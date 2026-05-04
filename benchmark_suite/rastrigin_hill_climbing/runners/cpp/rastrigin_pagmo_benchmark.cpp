#include <chrono>
#include <ctime>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>

#include <pagmo/algorithm.hpp>
#include <pagmo/algorithms/compass_search.hpp>
#include <pagmo/population.hpp>
#include <pagmo/problem.hpp>
#include <pagmo/problems/rastrigin.hpp>

namespace {

std::string json_escape(const std::string &value)
{
    std::ostringstream escaped;
    for (char ch : value) {
        switch (ch) {
        case '"':
            escaped << "\\\"";
            break;
        case '\\':
            escaped << "\\\\";
            break;
        case '\n':
            escaped << "\\n";
            break;
        case '\r':
            escaped << "\\r";
            break;
        case '\t':
            escaped << "\\t";
            break;
        default:
            escaped << ch;
            break;
        }
    }
    return escaped.str();
}

std::string json_string(const std::string &value)
{
    return std::string("\"") + json_escape(value) + "\"";
}

template <typename T>
T parse_value(const char *text, const char *name)
{
    std::istringstream input(text);
    T value;
    input >> value;
    if (!input || !input.eof()) {
        throw std::invalid_argument(std::string("invalid ") + name + ": " + text);
    }
    return value;
}

std::string format_vector(const pagmo::vector_double &values)
{
    std::ostringstream output;
    output << '[';
    output << std::setprecision(17);
    for (pagmo::vector_double::size_type index = 0; index < values.size(); ++index) {
        if (index > 0u) {
            output << ", ";
        }
        output << values[index];
    }
    output << ']';
    return output.str();
}

} // namespace

int main(int argc, char **argv)
{
    if (argc != 12) {
        std::cerr << "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <dimension> <budgetType> <budgetValue> <startRange> <stopRange> <reductionCoeff> <seed>\n";
        return 1;
    }

    try {
        const std::string benchmark_id = argv[1];
        const std::string algorithm_family = argv[2];
        const std::string problem_name = argv[3];
        const std::string instance_id = argv[4];
        const auto dimension = parse_value<unsigned>(argv[5], "dimension");
        const std::string budget_type = argv[6];
        const auto budget_value = parse_value<unsigned>(argv[7], "budgetValue");
        const auto start_range = parse_value<double>(argv[8], "startRange");
        const auto stop_range = parse_value<double>(argv[9], "stopRange");
        const auto reduction_coeff = parse_value<double>(argv[10], "reductionCoeff");
        const auto seed = parse_value<unsigned>(argv[11], "seed");

        if (budget_type != "evaluations") {
            throw std::invalid_argument("only evaluation budgets are supported");
        }

        pagmo::problem problem{pagmo::rastrigin(dimension)};
        pagmo::population population{problem, 1u, seed};
        pagmo::compass_search uda{budget_value, start_range, stop_range, reduction_coeff};
        uda.set_verbosity(0u);
        pagmo::algorithm algorithm{uda};

        const auto cpu_start = std::clock();
        const auto wall_start = std::chrono::steady_clock::now();
        population = algorithm.evolve(population);
        const auto wall_end = std::chrono::steady_clock::now();
        const auto cpu_end = std::clock();

        const auto wall_time_ms =
            std::chrono::duration<double, std::milli>(wall_end - wall_start).count();
        const auto cpu_time_ms =
            1000.0 * static_cast<double>(cpu_end - cpu_start) / static_cast<double>(CLOCKS_PER_SEC);

        const auto &best_solution = population.champion_x();
        const auto &best_fitness = population.champion_f();

        std::cout << std::setprecision(17);
        std::cout << "{\n";
        std::cout << "  \"benchmark_id\": " << json_string(benchmark_id) << ",\n";
        std::cout << "  \"library\": \"pagmo2_cpp\",\n";
        std::cout << "  \"algorithm_family\": " << json_string(algorithm_family) << ",\n";
        std::cout << "  \"problem\": " << json_string(problem_name) << ",\n";
        std::cout << "  \"instance_id\": " << json_string(instance_id) << ",\n";
        std::cout << "  \"seed\": " << seed << ",\n";
        std::cout << "  \"budget_type\": " << json_string(budget_type) << ",\n";
        std::cout << "  \"budget_value\": " << budget_value << ",\n";
        std::cout << "  \"best_fitness\": " << best_fitness.at(0) << ",\n";
        std::cout << "  \"best_solution\": " << format_vector(best_solution) << ",\n";
        std::cout << "  \"wall_time_ms\": " << wall_time_ms << ",\n";
        std::cout << "  \"cpu_time_ms\": " << cpu_time_ms << ",\n";
        std::cout << "  \"status\": \"ok\",\n";
        std::cout << "  \"error\": null\n";
        std::cout << "}\n";
    } catch (const std::exception &error) {
        std::cerr << error.what() << '\n';
        return 2;
    }

    return 0;
}

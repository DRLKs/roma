#include <chrono>
#include <cmath>
#include <ctime>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include <pagmo/algorithm.hpp>
#include <pagmo/algorithms/de.hpp>
#include <pagmo/population.hpp>
#include <pagmo/problem.hpp>
#include <pagmo/types.hpp>

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

class ackley_problem {
public:
    ackley_problem() = default;

    ackley_problem(unsigned dimension, double lower_bound, double upper_bound)
        : m_dimension(dimension), m_lower_bound(lower_bound), m_upper_bound(upper_bound)
    {
        if (m_dimension == 0u) {
            throw std::invalid_argument("dimension must be positive");
        }
        if (!(m_lower_bound < m_upper_bound)) {
            throw std::invalid_argument("lower_bound must be smaller than upper_bound");
        }
    }

    pagmo::vector_double fitness(const pagmo::vector_double &x) const
    {
        const auto dimension = static_cast<double>(x.size());
        double squared_mean = 0.0;
        double cosine_mean = 0.0;
        for (double value : x) {
            squared_mean += value * value;
            cosine_mean += std::cos(2.0 * std::acos(-1.0) * value);
        }
        squared_mean /= dimension;
        cosine_mean /= dimension;
        const double objective = -20.0 * std::exp(-0.2 * std::sqrt(squared_mean))
            - std::exp(cosine_mean) + 20.0 + std::exp(1.0);
        return {objective};
    }

    std::pair<pagmo::vector_double, pagmo::vector_double> get_bounds() const
    {
        return {
            pagmo::vector_double(m_dimension, m_lower_bound),
            pagmo::vector_double(m_dimension, m_upper_bound),
        };
    }

    std::string get_name() const
    {
        return "Ackley";
    }

private:
    unsigned m_dimension;
    double m_lower_bound;
    double m_upper_bound;
};

} // namespace

int main(int argc, char **argv)
{
    if (argc != 15) {
        std::cerr << "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <dimension> <lowerBound> <upperBound> <budgetType> <budgetValue> <populationSize> <crossoverRate> <differentialWeight> <variant> <seed>\n";
        return 1;
    }

    try {
        const std::string benchmark_id = argv[1];
        const std::string algorithm_family = argv[2];
        const std::string problem_name = argv[3];
        const std::string instance_id = argv[4];
        const auto dimension = parse_value<unsigned>(argv[5], "dimension");
        const auto lower_bound = parse_value<double>(argv[6], "lowerBound");
        const auto upper_bound = parse_value<double>(argv[7], "upperBound");
        const std::string budget_type = argv[8];
        const auto budget_value = parse_value<unsigned>(argv[9], "budgetValue");
        const auto population_size = parse_value<unsigned>(argv[10], "populationSize");
        const auto crossover_rate = parse_value<double>(argv[11], "crossoverRate");
        const auto differential_weight = parse_value<double>(argv[12], "differentialWeight");
        const auto variant = parse_value<unsigned>(argv[13], "variant");
        const auto seed = parse_value<unsigned>(argv[14], "seed");

        if (budget_type != "evaluations") {
            throw std::invalid_argument("only evaluation budgets are supported");
        }
        if (population_size < 4u) {
            throw std::invalid_argument("population size must be >= 4");
        }
        if (budget_value < population_size) {
            throw std::invalid_argument("budget must be at least the population size");
        }

        const auto generations = budget_value / population_size > 0u
            ? (budget_value / population_size) - 1u
            : 0u;

        pagmo::problem problem{ackley_problem(dimension, lower_bound, upper_bound)};
        pagmo::population population{problem, population_size, seed};
        pagmo::de uda{generations, differential_weight, crossover_rate, variant, 0.0, 0.0, seed};
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
        std::cout << "  \"evaluations\": " << budget_value << ",\n";
        std::cout << "  \"status\": \"ok\",\n";
        std::cout << "  \"error\": null\n";
        std::cout << "}\n";
    } catch (const std::exception &error) {
        std::cerr << error.what() << '\n';
        return 2;
    }

    return 0;
}

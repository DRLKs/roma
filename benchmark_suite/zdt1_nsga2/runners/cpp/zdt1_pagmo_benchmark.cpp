#include <algorithm>
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
#include <pagmo/algorithms/nsga2.hpp>
#include <pagmo/population.hpp>
#include <pagmo/problem.hpp>
#include <pagmo/problems/zdt.hpp>

namespace {

struct pareto_point {
    std::vector<double> variables;
    std::vector<double> objectives;
};

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

std::vector<double> parse_reference_point(const std::string &text)
{
    std::string trimmed = text;
    trimmed.erase(std::remove_if(trimmed.begin(), trimmed.end(), ::isspace), trimmed.end());
    if (trimmed.size() < 2u || trimmed.front() != '[' || trimmed.back() != ']') {
        throw std::invalid_argument("referencePoint must be a JSON array");
    }
    trimmed = trimmed.substr(1u, trimmed.size() - 2u);
    if (trimmed.empty()) {
        return {};
    }

    std::vector<double> values;
    std::stringstream stream(trimmed);
    std::string token;
    while (std::getline(stream, token, ',')) {
        values.push_back(parse_value<double>(token.c_str(), "referencePointItem"));
    }
    return values;
}

std::string format_double_vector(const std::vector<double> &values)
{
    std::ostringstream output;
    output << '[';
    for (std::size_t index = 0; index < values.size(); ++index) {
        if (index > 0u) {
            output << ", ";
        }
        output << values[index];
    }
    output << ']';
    return output.str();
}

std::string format_pareto_front(const std::vector<pareto_point> &front)
{
    std::ostringstream output;
    output << '[';
    for (std::size_t index = 0; index < front.size(); ++index) {
        if (index > 0u) {
            output << ", ";
        }
        output << "{\"variables\": " << format_double_vector(front[index].variables)
               << ", \"objectives\": " << format_double_vector(front[index].objectives)
               << '}';
    }
    output << ']';
    return output.str();
}

bool dominates(const std::vector<double> &left, const std::vector<double> &right)
{
    bool strictly_better = false;
    for (std::size_t index = 0; index < left.size(); ++index) {
        if (left[index] > right[index]) {
            return false;
        }
        if (left[index] < right[index]) {
            strictly_better = true;
        }
    }
    return strictly_better;
}

std::vector<pareto_point> non_dominated_front(
    const std::vector<pagmo::vector_double> &variables,
    const std::vector<pagmo::vector_double> &objectives)
{
    std::vector<pareto_point> front;
    for (std::size_t index = 0; index < objectives.size(); ++index) {
        bool dominated_point = false;
        for (std::size_t other_index = 0; other_index < objectives.size(); ++other_index) {
            if (index == other_index) {
                continue;
            }
            if (dominates(objectives[other_index], objectives[index])) {
                dominated_point = true;
                break;
            }
        }
        if (!dominated_point) {
            front.push_back({variables[index], objectives[index]});
        }
    }

    std::sort(front.begin(), front.end(), [](const pareto_point &left, const pareto_point &right) {
        if (left.objectives[0] == right.objectives[0]) {
            return left.objectives[1] < right.objectives[1];
        }
        return left.objectives[0] < right.objectives[0];
    });
    return front;
}

double hypervolume_2d(const std::vector<pareto_point> &front, const std::vector<double> &reference_point)
{
    std::vector<std::pair<double, double>> filtered;
    for (const auto &point : front) {
        if (point.objectives.size() != 2u) {
            continue;
        }
        const auto f1 = point.objectives[0];
        const auto f2 = point.objectives[1];
        if (f1 <= reference_point[0] && f2 <= reference_point[1]) {
            filtered.emplace_back(f1, f2);
        }
    }

    std::sort(filtered.begin(), filtered.end());

    double total = 0.0;
    double previous_f2 = reference_point[1];
    for (const auto &[f1, f2] : filtered) {
        if (f2 < previous_f2) {
            total += std::max(0.0, reference_point[0] - f1) * (previous_f2 - f2);
            previous_f2 = f2;
        }
    }
    return total;
}

unsigned generations_from_budget(unsigned budget_value, unsigned population_size)
{
    if (population_size == 0u) {
        throw std::invalid_argument("population size must be positive");
    }
    if (budget_value <= population_size) {
        return 1u;
    }
    return std::max(1u, (budget_value - population_size) / population_size);
}

} // namespace

int main(int argc, char **argv)
{
    if (argc != 15) {
        std::cerr << "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <dimension> <budgetType> <budgetValue> <populationSize> <crossoverProbability> <mutationProbability> <etaC> <etaM> <referencePointJson> <seed>\n";
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
        const auto population_size = parse_value<unsigned>(argv[8], "populationSize");
        const auto crossover_probability = parse_value<double>(argv[9], "crossoverProbability");
        const auto mutation_probability = parse_value<double>(argv[10], "mutationProbability");
        const auto eta_c = parse_value<double>(argv[11], "etaC");
        const auto eta_m = parse_value<double>(argv[12], "etaM");
        const auto reference_point = parse_reference_point(argv[13]);
        const auto seed = parse_value<unsigned>(argv[14], "seed");

        if (budget_type != "evaluations") {
            throw std::invalid_argument("only evaluation budgets are supported");
        }
        if (reference_point.size() != 2u) {
            throw std::invalid_argument("referencePoint must contain two values");
        }

        const auto generations = generations_from_budget(budget_value, population_size);
        pagmo::problem problem{pagmo::zdt(1u, dimension)};
        pagmo::population population{problem, population_size, seed};
        pagmo::algorithm algorithm{pagmo::nsga2(generations, crossover_probability, eta_c, mutation_probability, eta_m, seed)};

        const auto wall_start = std::chrono::steady_clock::now();
        const auto cpu_start = std::clock();
        population = algorithm.evolve(population);
        const auto cpu_end = std::clock();
        const auto wall_end = std::chrono::steady_clock::now();

        const auto variables = population.get_x();
        const auto objectives = population.get_f();
        const auto pareto_front = non_dominated_front(variables, objectives);
        const auto hypervolume = hypervolume_2d(pareto_front, reference_point);
        const auto wall_time_ms =
            std::chrono::duration_cast<std::chrono::duration<double, std::milli>>(wall_end - wall_start).count();
        const auto cpu_time_ms = 1000.0 * static_cast<double>(cpu_end - cpu_start) / static_cast<double>(CLOCKS_PER_SEC);

        std::ostringstream output;
        output << std::setprecision(17);
        output << "{\n";
        output << "  \"benchmark_id\": " << json_string(benchmark_id) << ",\n";
        output << "  \"library\": \"pagmo2_cpp\",\n";
        output << "  \"algorithm_family\": " << json_string(algorithm_family) << ",\n";
        output << "  \"problem\": " << json_string(problem_name) << ",\n";
        output << "  \"instance_id\": " << json_string(instance_id) << ",\n";
        output << "  \"seed\": " << seed << ",\n";
        output << "  \"budget_type\": " << json_string(budget_type) << ",\n";
        output << "  \"budget_value\": " << budget_value << ",\n";
        output << "  \"result_metric_name\": \"hypervolume\",\n";
        output << "  \"final_fitness\": " << hypervolume << ",\n";
        output << "  \"best_fitness\": " << hypervolume << ",\n";
        output << "  \"best_solution\": null,\n";
        output << "  \"pareto_front\": " << format_pareto_front(pareto_front) << ",\n";
        output << "  \"convergence_history\": [[" << budget_value << ", " << hypervolume << "]],\n";
        output << "  \"wall_time_ms\": " << wall_time_ms << ",\n";
        output << "  \"cpu_time_ms\": " << cpu_time_ms << ",\n";
        output << "  \"evaluations\": " << budget_value << ",\n";
        output << "  \"status\": \"ok\",\n";
        output << "  \"error\": null\n";
        output << "}";

        std::cout << output.str() << std::endl;
    } catch (const std::exception &error) {
        std::cerr << error.what() << std::endl;
        return 2;
    }

    return 0;
}
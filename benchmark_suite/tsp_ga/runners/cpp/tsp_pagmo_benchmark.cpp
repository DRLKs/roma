#include <algorithm>
#include <chrono>
#include <cmath>
#include <ctime>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <numeric>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include <pagmo/algorithm.hpp>
#include <pagmo/algorithms/sga.hpp>
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

std::string format_usize_vector(const std::vector<std::size_t> &values)
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

std::vector<std::pair<double, double>> read_tsplib_coordinates(const std::string &path)
{
    std::ifstream input(path);
    if (!input) {
        throw std::runtime_error("failed to open TSPLIB instance: " + path);
    }

    std::vector<std::pair<double, double>> coordinates;
    std::string line;
    bool in_section = false;

    while (std::getline(input, line)) {
        if (line.empty()) {
            continue;
        }

        if (!in_section) {
            if (line.find("NODE_COORD_SECTION") != std::string::npos) {
                in_section = true;
            }
            continue;
        }

        if (line.find("EOF") != std::string::npos) {
            break;
        }

        std::istringstream row(line);
        unsigned index = 0u;
        double x = 0.0;
        double y = 0.0;
        if (!(row >> index >> x >> y)) {
            continue;
        }
        coordinates.emplace_back(x, y);
    }

    if (coordinates.empty()) {
        throw std::runtime_error("TSPLIB instance did not contain coordinates");
    }

    return coordinates;
}

double rounded_euclidean_distance(
    const std::pair<double, double> &a,
    const std::pair<double, double> &b)
{
    return std::round(std::hypot(a.first - b.first, a.second - b.second));
}

std::vector<std::vector<double>> build_distance_matrix(
    const std::vector<std::pair<double, double>> &coordinates)
{
    const auto size = coordinates.size();
    std::vector<std::vector<double>> matrix(size, std::vector<double>(size, 0.0));

    for (std::size_t i = 0; i < size; ++i) {
        for (std::size_t j = i + 1; j < size; ++j) {
            const auto distance = rounded_euclidean_distance(coordinates[i], coordinates[j]);
            matrix[i][j] = distance;
            matrix[j][i] = distance;
        }
    }

    return matrix;
}

std::vector<std::size_t> decode_route(const pagmo::vector_double &keys)
{
    std::vector<std::size_t> route(keys.size());
    std::iota(route.begin(), route.end(), 0u);
    std::stable_sort(route.begin(), route.end(), [&keys](std::size_t lhs, std::size_t rhs) {
        if (keys[lhs] == keys[rhs]) {
            return lhs < rhs;
        }
        return keys[lhs] < keys[rhs];
    });
    return route;
}

double route_distance(
    const std::vector<std::size_t> &route,
    const std::vector<std::vector<double>> &distance_matrix)
{
    if (route.size() < 2u) {
        return 0.0;
    }

    double total = 0.0;
    for (std::size_t index = 0; index + 1u < route.size(); ++index) {
        total += distance_matrix[route[index]][route[index + 1u]];
    }
    total += distance_matrix[route.back()][route.front()];
    return total;
}

struct tsp_random_keys_problem {
    tsp_random_keys_problem() = default;

    explicit tsp_random_keys_problem(std::vector<std::vector<double>> matrix)
        : m_distance_matrix(std::move(matrix))
    {
    }

    pagmo::vector_double fitness(const pagmo::vector_double &x) const
    {
        const auto route = decode_route(x);
        return {route_distance(route, m_distance_matrix)};
    }

    std::pair<pagmo::vector_double, pagmo::vector_double> get_bounds() const
    {
        const auto dimension = m_distance_matrix.size();
        return {
            pagmo::vector_double(dimension, 0.0),
            pagmo::vector_double(dimension, 1.0),
        };
    }

    std::string get_name() const
    {
        return "TSP Random Keys";
    }

private:
    std::vector<std::vector<double>> m_distance_matrix;
};

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

bool is_time_budget(const std::string &budget_type)
{
    return budget_type == "time";
}

} // namespace

int main(int argc, char **argv)
{
    if (argc != 18) {
        std::cerr << "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <tsplibPath> <budgetType> <budgetValue> <populationSize> <crossoverProbability> <mutationProbability> <etaC> <paramM> <selection> <crossover> <mutation> <selectionParam> <seed>\n";
        return 1;
    }

    try {
        const std::string benchmark_id = argv[1];
        const std::string algorithm_family = argv[2];
        const std::string problem_name = argv[3];
        const std::string instance_id = argv[4];
        const std::string tsp_path = argv[5];
        const std::string budget_type = argv[6];
        const auto budget_value = parse_value<unsigned>(argv[7], "budgetValue");
        const auto population_size = parse_value<unsigned>(argv[8], "populationSize");
        const auto crossover_probability = parse_value<double>(argv[9], "crossoverProbability");
        const auto mutation_probability = parse_value<double>(argv[10], "mutationProbability");
        const auto eta_c = parse_value<double>(argv[11], "etaC");
        const auto param_m = parse_value<double>(argv[12], "paramM");
        const std::string selection = argv[13];
        const std::string crossover = argv[14];
        const std::string mutation = argv[15];
        const auto selection_param = parse_value<unsigned>(argv[16], "selectionParam");
        const auto seed = parse_value<unsigned>(argv[17], "seed");

        if (budget_type != "evaluations" && budget_type != "time") {
            throw std::invalid_argument("only evaluation or time budgets are supported");
        }
        if (budget_value == 0u) {
            throw std::invalid_argument("budgetValue must be positive");
        }

        const auto coordinates = read_tsplib_coordinates(tsp_path);
        const auto distance_matrix = build_distance_matrix(coordinates);
        const auto generations =
            is_time_budget(budget_type) ? 1u : generations_from_budget(budget_value, population_size);

        pagmo::problem problem{tsp_random_keys_problem(distance_matrix)};
        pagmo::population population{problem, population_size, seed};
        pagmo::sga uda{
            generations,
            crossover_probability,
            eta_c,
            mutation_probability,
            param_m,
            selection_param,
            crossover,
            mutation,
            selection,
            seed,
        };
        uda.set_verbosity(0u);
        pagmo::algorithm algorithm{uda};

        const auto cpu_start = std::clock();
        const auto wall_start = std::chrono::steady_clock::now();
        if (is_time_budget(budget_type)) {
            const auto deadline = wall_start + std::chrono::seconds(budget_value);
            bool ran_once = false;
            while (!ran_once || std::chrono::steady_clock::now() < deadline) {
                population = algorithm.evolve(population);
                ran_once = true;
            }
        } else {
            population = algorithm.evolve(population);
        }
        const auto wall_end = std::chrono::steady_clock::now();
        const auto cpu_end = std::clock();

        const auto wall_time_ms =
            std::chrono::duration<double, std::milli>(wall_end - wall_start).count();
        const auto cpu_time_ms =
            1000.0 * static_cast<double>(cpu_end - cpu_start) / static_cast<double>(CLOCKS_PER_SEC);

        const auto &best_keys = population.champion_x();
        const auto best_route = decode_route(best_keys);
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
        std::cout << "  \"best_solution\": " << format_usize_vector(best_route) << ",\n";
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
import java.lang.management.ManagementFactory;
import java.lang.management.ThreadMXBean;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;
import java.util.logging.Level;
import org.uma.jmetal.algorithm.Algorithm;
import org.uma.jmetal.algorithm.multiobjective.nsgaii.NSGAIIBuilder;
import org.uma.jmetal.operator.crossover.impl.SBXCrossover;
import org.uma.jmetal.operator.mutation.impl.PolynomialMutation;
import org.uma.jmetal.operator.selection.impl.BinaryTournamentSelection;
import org.uma.jmetal.problem.doubleproblem.DoubleProblem;
import org.uma.jmetal.problem.multiobjective.zdt.ZDT1;
import org.uma.jmetal.solution.doublesolution.DoubleSolution;
import org.uma.jmetal.util.JMetalLogger;
import org.uma.jmetal.util.pseudorandom.JMetalRandom;

public final class Zdt1NsgaIIBenchmark {
  private Zdt1NsgaIIBenchmark() {}

  private static final class ParetoPoint {
    private final List<Double> variables;
    private final List<Double> objectives;

    private ParetoPoint(List<Double> variables, List<Double> objectives) {
      this.variables = variables;
      this.objectives = objectives;
    }
  }

  private static String jsonEscape(String value) {
    StringBuilder escaped = new StringBuilder(value.length());
    for (int index = 0; index < value.length(); index++) {
      char ch = value.charAt(index);
      switch (ch) {
        case '"':
          escaped.append("\\\"");
          break;
        case '\\':
          escaped.append("\\\\");
          break;
        case '\n':
          escaped.append("\\n");
          break;
        case '\r':
          escaped.append("\\r");
          break;
        case '\t':
          escaped.append("\\t");
          break;
        default:
          escaped.append(ch);
      }
    }
    return escaped.toString();
  }

  private static String jsonString(String value) {
    return "\"" + jsonEscape(value) + "\"";
  }

  private static String formatDoubleList(List<Double> values) {
    StringBuilder rendered = new StringBuilder("[");
    for (int index = 0; index < values.size(); index++) {
      if (index > 0) {
        rendered.append(", ");
      }
      rendered.append(Double.toString(values.get(index)));
    }
    rendered.append(']');
    return rendered.toString();
  }

  private static String formatParetoFront(List<ParetoPoint> front) {
    StringBuilder rendered = new StringBuilder("[");
    for (int index = 0; index < front.size(); index++) {
      ParetoPoint point = front.get(index);
      if (index > 0) {
        rendered.append(", ");
      }
      rendered
          .append("{\"variables\": ")
          .append(formatDoubleList(point.variables))
          .append(", \"objectives\": ")
          .append(formatDoubleList(point.objectives))
          .append("}");
    }
    rendered.append(']');
    return rendered.toString();
  }

  private static String formatOptionalDouble(Double value) {
    return value == null ? "null" : Double.toString(value);
  }

  private static Double currentThreadCpuTimeMs(ThreadMXBean threadBean) {
    if (!threadBean.isCurrentThreadCpuTimeSupported()) {
      return null;
    }
    if (!threadBean.isThreadCpuTimeEnabled()) {
      try {
        threadBean.setThreadCpuTimeEnabled(true);
      } catch (UnsupportedOperationException ignored) {
        return null;
      }
    }
    return threadBean.getCurrentThreadCpuTime() / 1_000_000.0;
  }

  private static boolean dominates(List<Double> left, List<Double> right) {
    boolean strictlyBetter = false;
    for (int index = 0; index < left.size(); index++) {
      double leftValue = left.get(index);
      double rightValue = right.get(index);
      if (leftValue > rightValue) {
        return false;
      }
      if (leftValue < rightValue) {
        strictlyBetter = true;
      }
    }
    return strictlyBetter;
  }

  private static List<ParetoPoint> nonDominatedFront(List<DoubleSolution> population) {
    List<ParetoPoint> front = new ArrayList<>();
    for (int index = 0; index < population.size(); index++) {
      DoubleSolution candidate = population.get(index);
      List<Double> candidateObjectives = List.of(candidate.objectives()[0], candidate.objectives()[1]);
      boolean dominated = false;
      for (int otherIndex = 0; otherIndex < population.size(); otherIndex++) {
        if (otherIndex == index) {
          continue;
        }
        DoubleSolution other = population.get(otherIndex);
        List<Double> otherObjectives = List.of(other.objectives()[0], other.objectives()[1]);
        if (dominates(otherObjectives, candidateObjectives)) {
          dominated = true;
          break;
        }
      }
      if (!dominated) {
        front.add(new ParetoPoint(new ArrayList<>(candidate.variables()), candidateObjectives));
      }
    }
    front.sort(
        Comparator.comparing((ParetoPoint point) -> point.objectives.get(0))
            .thenComparing(point -> point.objectives.get(1)));
    return front;
  }

  private static List<Double> parseReferencePoint(String text) {
    String trimmed = text.trim();
    if (!trimmed.startsWith("[") || !trimmed.endsWith("]")) {
      throw new IllegalArgumentException("referencePoint must be a JSON array");
    }
    String body = trimmed.substring(1, trimmed.length() - 1).trim();
    List<Double> values = new ArrayList<>();
    if (body.isEmpty()) {
      return values;
    }
    for (String token : body.split(",")) {
      values.add(Double.parseDouble(token.trim()));
    }
    return values;
  }

  private static double hypervolume2d(List<ParetoPoint> front, List<Double> referencePoint) {
    List<ParetoPoint> filtered = new ArrayList<>();
    for (ParetoPoint point : front) {
      if (point.objectives.size() != 2) {
        continue;
      }
      if (point.objectives.get(0) <= referencePoint.get(0)
          && point.objectives.get(1) <= referencePoint.get(1)) {
        filtered.add(point);
      }
    }
    filtered.sort(
        Comparator.comparing((ParetoPoint point) -> point.objectives.get(0))
            .thenComparing(point -> point.objectives.get(1)));

    double total = 0.0;
    double previousF2 = referencePoint.get(1);
    for (ParetoPoint point : filtered) {
      double f1 = point.objectives.get(0);
      double f2 = point.objectives.get(1);
      if (f2 < previousF2) {
        total += Math.max(0.0, referencePoint.get(0) - f1) * (previousF2 - f2);
        previousF2 = f2;
      }
    }
    return total;
  }

  public static void main(String[] args) {
    Locale.setDefault(Locale.US);
    if (args.length != 15) {
      System.err.println(
          "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <dimension> <budgetType> <budgetValue> <populationSize> <offspringPopulationSize> <crossoverProbability> <mutationProbability> <sbxDistributionIndex> <polynomialDistributionIndex> <referencePointJson> <seed>");
      System.exit(1);
    }

    String benchmarkId = args[0];
    String algorithmFamily = args[1];
    String problemName = args[2];
    String instanceId = args[3];
    int dimension = Integer.parseInt(args[4]);
    String budgetType = args[5];
    int budgetValue = Integer.parseInt(args[6]);
    int populationSize = Integer.parseInt(args[7]);
    int offspringPopulationSize = Integer.parseInt(args[8]);
    double crossoverProbability = Double.parseDouble(args[9]);
    double mutationProbability = Double.parseDouble(args[10]);
    double sbxDistributionIndex = Double.parseDouble(args[11]);
    double polynomialDistributionIndex = Double.parseDouble(args[12]);
    List<Double> referencePoint = parseReferencePoint(args[13]);
    long seed = Long.parseLong(args[14]);

    if (!"evaluations".equals(budgetType)) {
      throw new IllegalArgumentException("Only evaluation budgets are supported");
    }

    JMetalLogger.logger.setLevel(Level.OFF);
    JMetalLogger.logger.setUseParentHandlers(false);
    JMetalRandom.getInstance().setSeed(seed);

    DoubleProblem problem = new ZDT1(dimension);
    SBXCrossover crossover = new SBXCrossover(crossoverProbability, sbxDistributionIndex);
    PolynomialMutation mutation = new PolynomialMutation(mutationProbability, polynomialDistributionIndex);

    Algorithm<List<DoubleSolution>> algorithm =
        new NSGAIIBuilder<>(problem, crossover, mutation, populationSize)
            .setOffspringPopulationSize(offspringPopulationSize)
            .setMaxEvaluations(budgetValue)
            .setSelectionOperator(new BinaryTournamentSelection<>())
            .build();

    ThreadMXBean threadBean = ManagementFactory.getThreadMXBean();
    Double cpuStartMs = currentThreadCpuTimeMs(threadBean);
    long wallStartNs = System.nanoTime();

    algorithm.run();
    List<DoubleSolution> population = algorithm.getResult();

    long wallEndNs = System.nanoTime();
    Double cpuEndMs = currentThreadCpuTimeMs(threadBean);
    Double cpuTimeMs = (cpuStartMs == null || cpuEndMs == null) ? null : cpuEndMs - cpuStartMs;
    List<ParetoPoint> paretoFront = nonDominatedFront(population);
    double hypervolume = hypervolume2d(paretoFront, referencePoint);
    double wallTimeMs = (wallEndNs - wallStartNs) / 1_000_000.0;

    StringBuilder payload = new StringBuilder();
    payload.append("{\n");
    payload.append("  \"benchmark_id\": ").append(jsonString(benchmarkId)).append(",\n");
    payload.append("  \"library\": \"jmetal_java\",\n");
    payload.append("  \"algorithm_family\": ").append(jsonString(algorithmFamily)).append(",\n");
    payload.append("  \"problem\": ").append(jsonString(problemName)).append(",\n");
    payload.append("  \"instance_id\": ").append(jsonString(instanceId)).append(",\n");
    payload.append("  \"seed\": ").append(seed).append(",\n");
    payload.append("  \"budget_type\": ").append(jsonString(budgetType)).append(",\n");
    payload.append("  \"budget_value\": ").append(budgetValue).append(",\n");
    payload.append("  \"result_metric_name\": \"hypervolume\",\n");
    payload.append("  \"final_fitness\": ").append(Double.toString(hypervolume)).append(",\n");
    payload.append("  \"best_fitness\": ").append(Double.toString(hypervolume)).append(",\n");
    payload.append("  \"best_solution\": null,\n");
    payload.append("  \"pareto_front\": ").append(formatParetoFront(paretoFront)).append(",\n");
    payload.append("  \"convergence_history\": [[").append(budgetValue).append(", ")
        .append(Double.toString(hypervolume)).append("]],\n");
    payload.append("  \"wall_time_ms\": ").append(Double.toString(wallTimeMs)).append(",\n");
    payload.append("  \"cpu_time_ms\": ").append(formatOptionalDouble(cpuTimeMs)).append(",\n");
    payload.append("  \"evaluations\": ").append(budgetValue).append(",\n");
    payload.append("  \"status\": \"ok\",\n");
    payload.append("  \"error\": null\n");
    payload.append("}");

    System.out.println(payload);
  }
}
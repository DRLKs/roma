import java.lang.management.ManagementFactory;
import java.lang.management.ThreadMXBean;
import java.util.Comparator;
import java.util.List;
import java.util.logging.Level;
import org.uma.jmetal.operator.localsearch.impl.BasicLocalSearch;
import org.uma.jmetal.operator.mutation.impl.PolynomialMutation;
import org.uma.jmetal.problem.singleobjective.Rastrigin;
import org.uma.jmetal.solution.doublesolution.DoubleSolution;
import org.uma.jmetal.util.JMetalLogger;
import org.uma.jmetal.util.comparator.ObjectiveComparator;
import org.uma.jmetal.util.pseudorandom.JMetalRandom;

public final class RastriginLocalSearchBenchmark {
  private RastriginLocalSearchBenchmark() {}

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

  public static void main(String[] args) {
    if (args.length != 10) {
      System.err.println(
          "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <dimension> <budgetType> <budgetValue> <mutationRate> <distributionIndex> <seed>");
      System.exit(1);
    }

    String benchmarkId = args[0];
    String algorithmFamily = args[1];
    String problemName = args[2];
    String instanceId = args[3];
    int dimension = Integer.parseInt(args[4]);
    String budgetType = args[5];
    int budgetValue = Integer.parseInt(args[6]);
    double mutationRate = Double.parseDouble(args[7]);
    double distributionIndex = Double.parseDouble(args[8]);
    long seed = Long.parseLong(args[9]);

    if (!"evaluations".equals(budgetType)) {
      System.err.println("Only evaluation budgets are supported");
      System.exit(2);
    }

    JMetalLogger.logger.setLevel(Level.OFF);
    JMetalLogger.logger.setUseParentHandlers(false);
    JMetalRandom.getInstance().setSeed(seed);

    var problem = new Rastrigin(dimension);
    var mutation = new PolynomialMutation(mutationRate, distributionIndex);
    Comparator<DoubleSolution> comparator = new ObjectiveComparator<>(0);

    DoubleSolution initialSolution = problem.createSolution();
    problem.evaluate(initialSolution);

    var localSearch = new BasicLocalSearch<DoubleSolution>(
      budgetValue,
      mutation,
      comparator,
      problem);

    ThreadMXBean threadBean = ManagementFactory.getThreadMXBean();
    Double cpuStartMs = currentThreadCpuTimeMs(threadBean);
    long wallStartNs = System.nanoTime();

    DoubleSolution foundSolution = localSearch.execute(initialSolution);

    long wallEndNs = System.nanoTime();
    Double cpuEndMs = currentThreadCpuTimeMs(threadBean);
    Double cpuTimeMs = (cpuStartMs == null || cpuEndMs == null) ? null : cpuEndMs - cpuStartMs;

    double bestFitness = foundSolution.objectives()[0];
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
    payload.append("  \"best_fitness\": ").append(Double.toString(bestFitness)).append(",\n");
    payload.append("  \"best_solution\": ").append(formatDoubleList(foundSolution.variables())).append(",\n");
    payload.append("  \"wall_time_ms\": ").append(Double.toString(wallTimeMs)).append(",\n");
    payload.append("  \"cpu_time_ms\": ").append(formatOptionalDouble(cpuTimeMs)).append(",\n");
    payload.append("  \"status\": \"ok\",\n");
    payload.append("  \"error\": null\n");
    payload.append("}");

    System.out.println(payload);
  }
}

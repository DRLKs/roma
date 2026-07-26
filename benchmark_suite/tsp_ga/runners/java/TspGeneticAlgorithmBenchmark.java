import java.lang.management.ManagementFactory;
import java.lang.management.ThreadMXBean;
import java.util.List;
import java.util.logging.Level;
import org.uma.jmetal.algorithm.Algorithm;
import org.uma.jmetal.algorithm.singleobjective.geneticalgorithm.GenerationalGeneticAlgorithm;
import org.uma.jmetal.algorithm.singleobjective.geneticalgorithm.GeneticAlgorithmBuilder;
import org.uma.jmetal.operator.crossover.impl.PMXCrossover;
import org.uma.jmetal.operator.mutation.impl.PermutationSwapMutation;
import org.uma.jmetal.operator.selection.impl.BinaryTournamentSelection;
import org.uma.jmetal.problem.singleobjective.TSP;
import org.uma.jmetal.solution.permutationsolution.PermutationSolution;
import org.uma.jmetal.util.JMetalLogger;
import org.uma.jmetal.util.evaluator.impl.SequentialSolutionListEvaluator;
import org.uma.jmetal.util.pseudorandom.JMetalRandom;

public final class TspGeneticAlgorithmBenchmark {
  private TspGeneticAlgorithmBenchmark() {}

  private static final class CountingTspProblem extends TSP {
    private int evaluationCount = 0;

    CountingTspProblem(String tspPath) throws java.io.IOException {
      super(tspPath);
    }

    @Override
    public PermutationSolution<Integer> evaluate(PermutationSolution<Integer> solution) {
      evaluationCount += 1;
      return super.evaluate(solution);
    }

    int getEvaluationCount() {
      return evaluationCount;
    }
  }

  private static final class TimeLimitedGenerationalTspGa
      extends GenerationalGeneticAlgorithm<PermutationSolution<Integer>> {
    private final long deadlineNs;

    TimeLimitedGenerationalTspGa(
        TSP problem,
        int populationSize,
        PMXCrossover crossover,
        PermutationSwapMutation mutation,
        BinaryTournamentSelection<PermutationSolution<Integer>> selection,
        long timeLimitMs) {
      super(
          problem,
          Integer.MAX_VALUE,
          populationSize,
          crossover,
          mutation,
          selection,
          new SequentialSolutionListEvaluator<>());
      this.deadlineNs = System.nanoTime() + Math.max(1L, timeLimitMs) * 1_000_000L;
    }

    @Override
    protected boolean isStoppingConditionReached() {
      return System.nanoTime() >= deadlineNs || super.isStoppingConditionReached();
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

  private static String formatIntegerList(List<Integer> values) {
    StringBuilder rendered = new StringBuilder("[");
    for (int index = 0; index < values.size(); index++) {
      if (index > 0) {
        rendered.append(", ");
      }
      rendered.append(Integer.toString(values.get(index)));
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

  public static void main(String[] args) throws Exception {
    if (args.length != 11) {
      System.err.println(
          "Usage: <benchmarkId> <algorithmFamily> <problem> <instanceId> <tsplibPath> <budgetType> <budgetValue> <populationSize> <crossoverProbability> <mutationProbability> <seed>");
      System.exit(1);
    }

    String benchmarkId = args[0];
    String algorithmFamily = args[1];
    String problemName = args[2];
    String instanceId = args[3];
    String tspPath = args[4];
    String budgetType = args[5];
    int budgetValue = Integer.parseInt(args[6]);
    int populationSize = Integer.parseInt(args[7]);
    double crossoverProbability = Double.parseDouble(args[8]);
    double mutationProbability = Double.parseDouble(args[9]);
    long seed = Long.parseLong(args[10]);

    if (!"evaluations".equals(budgetType) && !"time".equals(budgetType)) {
      System.err.println("Only evaluation or time budgets are supported");
      System.exit(2);
    }

    JMetalLogger.logger.setLevel(Level.OFF);
    JMetalLogger.logger.setUseParentHandlers(false);
    JMetalRandom.getInstance().setSeed(seed);

    CountingTspProblem problem = new CountingTspProblem(tspPath);
    PMXCrossover crossover = new PMXCrossover(crossoverProbability);
    PermutationSwapMutation mutation = new PermutationSwapMutation(mutationProbability);
    BinaryTournamentSelection<PermutationSolution<Integer>> selection = new BinaryTournamentSelection<>();

    Algorithm<PermutationSolution<Integer>> algorithm;
    if ("time".equals(budgetType)) {
      algorithm =
        new TimeLimitedGenerationalTspGa(
          problem,
          populationSize,
          crossover,
          mutation,
          selection,
          budgetValue * 1000L);
    } else {
      algorithm =
        new GeneticAlgorithmBuilder<>(problem, crossover, mutation)
          .setPopulationSize(populationSize)
          .setMaxEvaluations(budgetValue)
          .setSelectionOperator(selection)
          .setVariant(GeneticAlgorithmBuilder.GeneticAlgorithmVariant.GENERATIONAL)
          .build();
    }

    ThreadMXBean threadBean = ManagementFactory.getThreadMXBean();
    Double cpuStartMs = currentThreadCpuTimeMs(threadBean);
    long wallStartNs = System.nanoTime();

    algorithm.run();
    PermutationSolution<Integer> foundSolution = algorithm.getResult();

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
    payload.append("  \"best_solution\": ").append(formatIntegerList(foundSolution.variables())).append(",\n");
    payload.append("  \"wall_time_ms\": ").append(Double.toString(wallTimeMs)).append(",\n");
    payload.append("  \"cpu_time_ms\": ").append(formatOptionalDouble(cpuTimeMs)).append(",\n");
    payload.append("  \"evaluations\": ").append(problem.getEvaluationCount()).append(",\n");
    payload.append("  \"status\": \"ok\",\n");
    payload.append("  \"error\": null\n");
    payload.append("}");

    System.out.println(payload);
  }
}
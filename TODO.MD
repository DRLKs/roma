[X] 1. Refactorización de Traits y Genéricos (Sistema de Tipos)
    [X] Cambiar genéricos anidados (ej. `Algorithms<T, S, P>`) por Traits con Tipos Asociados (Associated Types). Ej: en el trait `Problem`, definir `type Variables;` y `type Fitness;`.
    [X] Aplanar la jerarquía de Traits. Dejar un único trait base `Algorithm` que defina el método de ejecución.

[X] 2. Estructura de la Solución (`Solution`)
    [X] Convertir `Solution` de un Trait a un Struct.
    [X] Eliminar las restricciones de traits (bounds como `<O: Ord>` o `Default`) en la definición del `struct Solution`, dejándolas solo en los bloques `impl` donde sea estrictamente necesario.

[X] 3. Adaptación a Optimización Multiobjetivo
    [X] Reemplazar `Set<f64>` por `Vec<f64>` para almacenar los objetivos (los f64 no implementan Eq ni Hash en Rust).
    [X] Eliminar la jerarquía orientada a objetos `QualityIndicator`.

[ ] 4. Operadores y Composición Estática
    [ ] Eliminar el uso de `Vec<Operator>` y `Box<dyn Operator>` (Dynamic Dispatch).
    [X] Inyectar los operadores concretos estáticamente mediante genéricos en el struct del algoritmo. Ej: `struct GeneticAlgorithm<P, S, C, M>`.

[ ] 5. Parámetros e Instanciación (Builders)
    [ ] Eliminar el struct monolítico `AlgorithmParameters`.
    [ ] Implementar el Patrón Builder para cada algoritmo específico. Esto permite configurar los parámetros particulares de cada uno (ej. `population_size`, `mutation_rate`) con validación en tiempo de compilación.

[ ] 6. Manejo de Eventos y Observadores
    [X] Eliminar el patrón Observer clásico (`Observable`, `AlgorithmObserver`) que obliga a manejar lifetimes o punteros compartidos complejos.
    [X] Reemplazar la notificación de progreso por el uso de Closures (`FnMut`) pasados al Builder, o canales de paso de mensajes (`std::sync::mpsc`).
    [ ] Hemos añadido más información. Cuando el algoritmo termina, termina debido a una condición de parada, esta condición de parada debe mostrarse en los charts o en lo repots o en ambos. Terminal, etc...

    [ ] Arreglar los observadores. Mejorar los HTML report observers

[X] 7. Memoria, requisitos
    [X] Desarrollar los casos de uso. Crear tus propios algoritmos y ejemplos o añadir el crate y usar los algoritmos de la biblioteca.
    [X] .....

[X] 8. Reproducibilidad 28/02/26

[ ] 9. Algoritmos y operadores, ideas de Tutor
    [X] Más criterios de paradas.
    [ ] Operadores, que devuelvan información para quizás hacer más rápidas las evaluaciones de las soluciones, porque devolverán información sobre lo que ha cambiado de la solución

[X] 10. Ideas, Observadores y Criterios de paradas. MUY CORE.  - - - Asignado para --> 09/03/2026 u 10/03/2026
    [X] Ambas estructuras almacenan información sobre el estado actual del algoritmo, quizás juntar esta información en una estructura más grande    
    ---Algoritmos
    [ ] Quizás quitar get_solution_set y get_solution_set
    [X] Mi idea general es que la función run, sea la misma para todos los algoritmos. 
        Hilo distinto para que los observadores estén en otro hilo {Validate parameters, while !should_terminate : step(), context.update }.
        Luego step cambia para cada algoritmo.
    [X] No me gustan los panics, quizás quitarlos y cambiarlos por Result<>
    [ ] Quizás si no tuvieramos que copiar los criterios de terminación estaría bastante bien
    [X] Cambiar validate_parameters para que devuelva un result o algo así con el mensaje de error en caso de que los parámetros estén mal

[ ] 11. CSVs, JSON, YAML. Quiero que también se pueda añadir datos desde distintos tipos de archivos. Solo está implementado para CSVs, que de hecho es el que peor funciona para estos casos.
    [X] Mejorar esta implementación, por ahora muy mala (Más o menos)
    [ ] Tiene margen de mejora

[ ] 12. Reestructurar el proyecto
    [X] ExecutionStateSnapshot no creo que deba estar en Termination.
    [ ] La estructura que manejamos de traits en cada módulo la vi de un repositorio de X(Twitter), pero no se si será lo mejor para esta librería.
 
[X] 13. Aunque sea duro decirlo, creo que debemos volver a mirar Solution. Para el multiobjetivo va bastante raro. Obtener la mejor solución es más aleatorio en estos casos que algo real. Se observa el primer objetivo, que quizás si estamos mirando Pareto tiene sentido, pero lo dudo enormemente

[ ] 14. Para el tema de los experimentos, debemos regular de alguna manera los parámetros de los algoritmos. Debe existir alguna manera de categorizar los parámetros para que sean más intuitivo y sencilla la creación de ejecuciones de pruebas y experimentos. Quizás que los parámetros implementen un trait o alguna otra cosa, supongo que existirá algo en Rust para solucionarlo.
    [ ] Añadir más formas para comparar las ejecuciones.
    [X] Crear observadores especiales para experimentos.
    [X] Quizás directamente que los algoritmos puedan implementar algo para que puedan probarse en experimentos. **

    ( lo implementado a mi parecer es muy verboso, debemos observarlo con cautela y modificarlo si fuera necesario )
    ( los observadores de los experimentos creo que están bastante decentes, pero seguramente sean mejorables )

[ ] 15. Creación de macros para hacer más abstracta la biblioteca, esto quizás es demasiado avanzado

[X] 16. Experimentos.
    [X] Estandarizar los parámetros para que se puedan añadir a los summaries { Hecho a medias con el parser, es mejorable }
    [X] El algoritmo debe recibir parámetros, estos parámetros construir algoritmos y que estos se ejecuten

[X] 17. TAREA 12/03/2026: Pruebas + Limpiar código + Arquitectura
[ ] 18. Limpiar mucho el código 
[ ] 19. Los problemas multiobjetivo, son inresolvibles por algoritmo de mono-objetivo, quizás deberiamos hacer reestricciones para que estos algoritmos no puedan solucionarlos
[X] 20. Sin duda existen redundancias en la elección de que si el modelo es de minimización o de optimización. 
    [X]Quizás el compare solutions debería estar en problema, porque que una solución sea mejor que otra creo que depende más del problema que de la misma solución.

[X] 21. Hacer representaciones de las soluciones dependientemente del problema

[ ] 22. Desarrollar reportes sobre los experimentos

[ ] 23. Funcionamiento del almacenamiento de los checkpoints

    - run_algorithm toma -> ExecutionStateSnapshot ( Se almacenará en el archivo)
    - en vez de usar la seed del CLI o la estandar usar la almacenada 
        o (quizás eliminar la reproducibilidad)
    - EN caso de que el valor de la temrinación sea el tiempo deberemos hacer cálculos para obtener un nuevo tiempo de terminación que concuerde con el restante
    - Debemos guardar la solución, stepState. Crear una función que en vez del inicializar normal, sea por checkpoint
    - Cuando ocurre una ejecución, se van creando los checkpoints, pero cuando termine la ejecución y sea satisfactoria, el checkpoint debería borrarse
    - Cuando se resuma un checkpoint, se podría crear un checkpoint a partir de ese, desde el cual seguir la ejecución pero sin modificar el checkpoint anterior


    - En caso de los poblacionales

    [ ] Que funcione para los de población
    [ ] Que funcione para los experimentos

    [ ] Obtener los parámetros de cada problema especificamente para identificar correctamente los checkpoints

[ ] 24. Limpiar los checkoints LRU

[ ] 25. Realizar test unitarios Solution decode, encode

[ ] 26. Informar en los charts de que la ejecución se reaunudó
Cambiar algorithm.run añadir el argumento resume

[ ] 27. Creo que ExecutionStateSnapshot se puede eliminar

[X] 28. Buscar el error que tiene que tener TSP. Muy mal resultado muy poco tiempo de wall, no tiene que estar haciendo algo. Le faltan pasos seguro

[X] 29. Complicar los algoritmos

[ ] 30. Poner el flag --no-checkpoint a las ejecuciones

[ ] 31. En vez de compararlo con evaluaciones, vamos a compararlo con tiempo de ejecución (más realista)

[ ] 32. Analizar run_with_observers: Es limpiable muy facilmente

[ ] 33. Cambiar el orden en el que aparece INFO en checkpoint selection

[ ] 34. Limpiar el módulo CLI.

[ ] 35. Mover lógica de algoritmos de traits, no creo que sea el mejor sitio

[X] 36. Reducir tamaño imagen Workflows. Alpine o algo así

[ ] 37. Workflow o algo para subir releases y crates a la vez

[X] 38. Implementar checkpoint_algorithm_parameters para muchos algoritmos. No diferencia entonces por algoritmos con distintos operadores u parámetros de algoritmos

[ ] 39. Mirar bien CheckpointDirSource

[ ] 40. Objetivo:
Construir un pipeline completo y reproducible de benchmarking estadístico para comparar algoritmos/metaheurísticas estocásticas sobre múltiples problemas e instancias.

REQUISITOS GENERALES

* El benchmark debe ser científicamente correcto.
* Debe evitar pseudoreplicación.
* Debe permitir análisis estadísticos estándar en benchmarking de optimización.
* Debe ser reproducible.
* Debe soportar múltiples algoritmos, múltiples instancias y múltiples seeds.
* Debe separar claramente:

  * datos raw,
  * agregados,
  * análisis estadístico,
  * visualizaciones en latex

======================================================================

1. ESTRUCTURA EXPERIMENTAL
   ======================================================================

Para cada combinación:

(problem, dimension, algorithm)

ejecutar múltiples runs independientes usando distintas seeds.

Recomendación:

* 30 seeds por instancia.
* mismas seeds para todos los algoritmos.

Ejemplo:
seeds = [100..129]

IMPORTANTE:

* Las seeds NO son instancias independientes.
* Friedman NO debe aplicarse directamente sobre seeds.
* Las seeds solo sirven para estimar robustamente el rendimiento del algoritmo sobre una instancia.

Guardar UN registro por ejecución individual.

Formato recomendado CSV o Parquet.

Columnas mínimas:

problem
dimension
instance_id
algorithm
seed
budget_type
budget_value
final_fitness
wall_time_ms
cpu_time_ms
success
evaluations

Ejemplo:

rastrigin,80,rastrigin_dim80,roma,100,evaluations,3500,701.2,28.1,27.8,true,3500

Guardar también convergence history por run.

Formato recomendado:

evaluation,best_fitness_so_far

Ejemplo:
0,10000
100,8500
500,3000
3500,701

Esto permitirá:

* convergence plots,
* anytime analysis,
* AUC convergence,
* attainment analysis.

Para cada combinación:

(instance_id, algorithm)

agregar las múltiples seeds usando:

* mediana (preferido),
  o:
* mean robusta.

Calcular:

* mean
* median
* std
* min
* max
* p90
* IQR

IMPORTANTE:

* Friedman debe usar UN valor agregado por instancia.
* NO usar las seeds crudas directamente en Friedman.

Construir tabla:

instance_id | alg1 | alg2 | alg3

donde cada celda contiene:

* mediana sobre seeds.

Aplicar:

* Friedman test.

Hipótesis:
H0: todos los algoritmos tienen el mismo rendimiento.
H1: al menos uno difiere.

Obtener:

* statistic
* p-value
* average ranks

Si Friedman rechaza H0 (p < 0.05):

Aplicar:

* Holm correction
  o:
* Nemenyi posthoc.

Obtener:

* comparación pairwise,
* adjusted p-values.

Calcular:

* Vargha-Delaney A12
  o:
* Cliff's Delta.

Para pares relevantes de algoritmos.

Dentro de una instancia concreta:

Comparar algoritmos usando:

* Wilcoxon signed-rank test
  sobre seeds pareadas.

Como todos los algoritmos usan las mismas seeds:
(seed_i de alg A vs seed_i de alg B)

Esto sí es válido.

Analizar por separado:

A) Calidad:

* final_fitness

B) Tiempo:

* wall_time_ms
* cpu_time_ms

C) Trade-off:

* fitness vs tiempo

Generar automáticamente:

* boxplots por algoritmo,
* convergence plots,
* average rank tables,
* critical difference diagrams,
* scatter fitness vs tiempo.

benchmark/
raw/
aggregated/
analysis/
plots/
reports/

Generar automáticamente:

* tablas CSV,
* tablas LaTeX,
* Markdown report,
* JSON summaries,
* gráficos PNG/SVG,
* resultados estadísticos.

Usar:

* pandas
* numpy
* scipy
* scikit-posthocs
* matplotlib
* seaborn opcional
* pingouin opcional

Implementar:

A) Friedman:
scipy.stats.friedmanchisquare

B) Wilcoxon:
scipy.stats.wilcoxon

C) Posthoc:
scikit_posthocs.posthoc_nemenyi_friedman

D) Effect size:
Vargha-Delaney A12

* NO comparar solo mejores valores.
* NO comparar solo medias globales.
* NO aplicar Friedman directamente sobre seeds.
* NO mezclar seeds como si fueran instancias independientes.
* SIEMPRE usar múltiples instancias benchmark.
* SIEMPRE usar múltiples runs por instancia.
* SIEMPRE usar mismas seeds entre algoritmos.
* Mantener presupuesto idéntico semánticamente entre implementaciones.

El pipeline debe producir automáticamente:

1. Tabla descriptiva:
   algorithm | mean | median | std | best

2. Friedman:
   statistic + p-value

3. Average ranks:
   algorithm | avg_rank

4. Posthoc matrix:
   pairwise adjusted p-values

5. Effect sizes:
   A12 pairwise

6. Plots:

* boxplots
* convergence
* critical difference diagram

7. Report reproducible completo.

El resultado debe ser un benchmark científicamente defendible y cercano al estándar usado en papers de metaheurísticas y benchmarking de optimización estocástica.


# Decisiones de diseño - Dudas
Lo más complejo es evitar el pensamiento de clases de java y tener en mente como está diseñado Rust. 

No existe herencia, existen promesas o contratos, los traits

### Problema
Es la única estructura que conoce la finalidad del algoritmo, me explico, los límites, su objetivo, las conidicones que debe cumplir, etc...

### Algoritmo
Al algoritmo debe conocer el Problema. Un atributo del altgoritmo es el mismo problema, por tanto con este atributo podremos comprobar si las soluciones que obtiene el problema son correctas.

```Rust
pub trait HasProblem {
    fn problem(&self) -> &dyn Problem;
}

// Obligamos al algoritmo a tener un problema entre sus atributos
pub trait Algorithm: HasProblem {
    fn run(&mut self);
}

```

### Solucion
Al generarse muchísimas soluciones en un mismo algoritmo quizás no es muy útil que la solución contenga información sobre el problema. La idea es que la solución contenga un Quality indicator el cual indique lo correcta que es esa solución. 
Este indicador se pondrá automaticamente mediante una función `evaluate` en la estructura `Problema`.

##### SolutionInfo
Actualmente los traits Solution dependen de SolutionInfo, pero realmente no soy consciente de si este atributo tiene sentido y si es sobreingeniería (se podría usar un vector y ya). Por ahora vamos a dejar solutionInfo por si queremos añadir más atributos a solution
/// Trait that defines the basic interface for all optimization algorithms.
pub trait Algorithm {


    type Parameters;

    /// Runs the optimization algorithm.
    ///
    ///
    /// # Arguments
    /// 
    /// * `verbose` - Verbosity level of the output:
    ///   * `0` - No output
    ///   * `1` - Basic information (start, end)
    ///   * `>1` - Full debug information
    /// 
    fn run(&self, verbose: u8){

        match verbose {
            0 => {}, // No output
            1 => println!("Algorithm started..."),
            _ => println!("Algorithm running with detailed output..."),
        }
        
        if !self.validate_parameters(){
            panic!("Invalid parameters for the algorithm.");
        }


    }

    fn validate_parameters(&self) -> bool;

    fn get_parameters(&self) -> &Self::Parameters;

    fn set_parameters(&mut self, params: Self::Parameters);

    

}
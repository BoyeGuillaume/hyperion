/// Analysis statistics that can be used to gather information about behavior of
/// an block of instructions/function during execution or simulation.
pub enum AnalysisStatistic {
    /// Number of executed instructions, this is not tied to unique instructions,
    /// but rather counts how many instructions were executed in total.
    InstructionCount,
}

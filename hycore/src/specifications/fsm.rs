use hyinstr::modules::symbol::FunctionPointer;
use petgraph::graph::DiGraph;
use uuid::Uuid;

pub struct FinalStateSpecification {
    /// Uuid of this FSM specification
    pub uuid: Uuid,

    /// Graph structure representing the FSM
    pub fsm_graph: DiGraph<Uuid, ()>,

    /// Function referenced in this FSM
    pub referenced_functions: Vec<FunctionPointer>,
}

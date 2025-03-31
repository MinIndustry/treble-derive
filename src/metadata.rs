use super::parameters::Parameter;

#[derive(Clone, Debug, Default)]
pub struct FilterMetaData {
    // The name of the filter
    pub name: String,
    // The description of the filter
    pub description: String,
    // The number of input ports
    pub source_amount: usize,
    // The filter parameters
    pub parameters: Vec<Parameter>,
}

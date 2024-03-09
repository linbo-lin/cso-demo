use crate::metadata::MdAccessor;
use crate::operator::OperatorId;
use crate::{Demo, Plan};
use cso_core::expression::ScalarExpression;
use cso_core::metadata::Stats;
use cso_core::operator::LogicalOperator;
use cso_core::ColumnRefSet;
use std::rc::Rc;

#[derive(Debug)]
pub struct LogicalFilter {
    predicate: Rc<dyn ScalarExpression>,
}

impl LogicalFilter {
    pub fn new(predicate: Rc<dyn ScalarExpression>) -> Self {
        assert!(predicate.is_boolean_expression());
        LogicalFilter { predicate }
    }

    pub fn predicate(&self) -> &Rc<dyn ScalarExpression> {
        &self.predicate
    }
}

impl LogicalOperator<Demo> for LogicalFilter {
    fn name(&self) -> &str {
        "logical filter"
    }

    fn operator_id(&self) -> &OperatorId {
        &OperatorId::LogicalFilter
    }

    fn derive_statistics(&self, _md_accessor: &MdAccessor, input_stats: &[Rc<dyn Stats>]) -> Rc<dyn Stats> {
        input_stats[0].clone()
    }

    fn derive_output_columns(&self, inputs: &[Plan], column_set: &mut ColumnRefSet) {
        debug_assert_eq!(inputs.len(), 1);
        inputs[0].derive_output_columns(column_set);
    }
}

//! The core framework of cascade style optimizer

#![forbid(unsafe_code)]
#![allow(clippy::new_without_default)]

pub mod any;
pub mod cost;
pub mod expression;
pub mod memo;
pub mod metadata;
pub mod operator;
pub mod property;
pub mod rule;

mod task;

use crate::memo::{GroupPlanRef, Memo};
use crate::metadata::MdAccessor;
use crate::operator::{LogicalOperator, Operator, PhysicalOperator};
use crate::property::{LogicalProperties, PhysicalProperties};
use crate::rule::{RuleId, RuleSet};
use crate::task::{OptimizeGroupTask, TaskRunner};
use bit_set::BitSet;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

pub trait OptimizerType: 'static + PartialEq + Eq + Hash + Clone {
    type RuleId: RuleId;
    type OperatorId: PartialEq + Debug;
    type MdId: PartialEq + Eq + Clone + Hash + Debug + Serialize + for<'a> Deserialize<'a>;
}

pub struct LogicalPlan<T: OptimizerType> {
    op: Rc<dyn LogicalOperator<T>>,
    inputs: Vec<LogicalPlan<T>>,
    required_properties: Vec<PhysicalProperties<T>>,
}

impl<T: OptimizerType> LogicalPlan<T> {
    #[inline]
    pub const fn new(
        op: Rc<dyn LogicalOperator<T>>,
        inputs: Vec<LogicalPlan<T>>,
        required_properties: Vec<PhysicalProperties<T>>,
    ) -> Self {
        Self {
            op,
            inputs,
            required_properties,
        }
    }

    pub fn required_properties(&self) -> &[PhysicalProperties<T>] {
        &self.required_properties
    }
}

#[derive(Debug)]
pub struct PhysicalPlan<T: OptimizerType> {
    op: Rc<dyn PhysicalOperator<T>>,
    inputs: Vec<PhysicalPlan<T>>,
}

impl<T: OptimizerType> PhysicalPlan<T> {
    pub const fn new(op: Rc<dyn PhysicalOperator<T>>, inputs: Vec<PhysicalPlan<T>>) -> Self {
        PhysicalPlan { op, inputs }
    }

    pub fn operator(&self) -> &Rc<dyn PhysicalOperator<T>> {
        &self.op
    }

    pub fn inputs(&self) -> &[PhysicalPlan<T>] {
        &self.inputs
    }
}

impl<T: OptimizerType> PartialEq<Self> for PhysicalPlan<T> {
    fn eq(&self, other: &Self) -> bool {
        self.op.equal(other.op.as_ref()) && self.inputs.eq(other.inputs())
    }
}

#[derive(Clone)]
pub struct Plan<T: OptimizerType> {
    op: Operator<T>,
    inputs: Vec<Plan<T>>,
    _property: LogicalProperties,
    group_plan: Option<GroupPlanRef<T>>,
    _required_properties: Vec<PhysicalProperties<T>>,
}

impl<T: OptimizerType> Plan<T> {
    pub fn new(op: Operator<T>, inputs: Vec<Plan<T>>, group_plan: Option<GroupPlanRef<T>>) -> Self {
        Plan {
            op,
            inputs,
            _property: LogicalProperties {},
            group_plan,
            _required_properties: vec![],
        }
    }

    pub fn inputs(&self) -> &[Plan<T>] {
        &self.inputs
    }

    pub fn group_plan(&self) -> Option<&GroupPlanRef<T>> {
        self.group_plan.as_ref()
    }

    pub fn operator(&self) -> &Operator<T> {
        &self.op
    }

    /// Returns the columns in the table needed for the current plan.
    pub fn derive_output_columns(&self, column_set: &mut ColumnRefSet) {
        self.op.logical_op().derive_output_columns(&self.inputs, column_set)
    }
}

#[derive(Default)]
pub struct Options {}

pub struct Optimizer<T: OptimizerType> {
    _options: Options,
    _mark: PhantomData<T>,
}

impl<T: OptimizerType> Optimizer<T> {
    pub fn new(_options: Options) -> Optimizer<T> {
        Optimizer {
            _options,
            _mark: PhantomData,
        }
    }

    pub fn optimize(
        &mut self,
        plan: LogicalPlan<T>,
        required_properties: Rc<PhysicalProperties<T>>,
        md_accessor: MdAccessor<T>,
        rule_set: RuleSet<T>,
    ) -> PhysicalPlan<T> {
        let mut optimizer_ctx = OptimizerContext::new(md_accessor, rule_set);
        optimizer_ctx.memo_mut().init(plan);
        let mut task_runner = TaskRunner::new();
        let initial_task =
            OptimizeGroupTask::new(optimizer_ctx.memo().root_group().clone(), required_properties.clone());
        task_runner.push_task(initial_task);
        task_runner.run(&mut optimizer_ctx);
        optimizer_ctx.memo().extract_best_plan(&required_properties)
    }
}

pub struct OptimizerContext<T: OptimizerType> {
    memo: Memo<T>,
    rule_set: RuleSet<T>,
    md_accessor: MdAccessor<T>,
}

impl<T: OptimizerType> OptimizerContext<T> {
    fn new(md_accessor: MdAccessor<T>, rule_set: RuleSet<T>) -> Self {
        OptimizerContext {
            memo: Memo::new(),
            md_accessor,
            rule_set,
        }
    }

    pub fn memo_mut(&mut self) -> &mut Memo<T> {
        &mut self.memo
    }

    pub fn memo(&self) -> &Memo<T> {
        &self.memo
    }

    pub fn rule_set_mut(&mut self) -> &mut RuleSet<T> {
        &mut self.rule_set
    }

    pub fn rule_set(&self) -> &RuleSet<T> {
        &self.rule_set
    }

    pub fn md_accessor(&self) -> &MdAccessor<T> {
        &self.md_accessor
    }
}

#[repr(transparent)]
pub struct ColumnRefSet {
    bit_set: BitSet,
}

impl ColumnRefSet {
    pub fn new() -> Self {
        ColumnRefSet { bit_set: BitSet::new() }
    }

    pub fn with_id(id: u32) -> ColumnRefSet {
        let mut col_set = ColumnRefSet::new();
        col_set.insert(id);
        col_set
    }

    pub fn contains(&self, id: u32) -> bool {
        self.bit_set.contains(id as usize)
    }

    pub fn insert(&mut self, id: u32) -> bool {
        self.bit_set.insert(id as usize)
    }

    pub fn remove(&mut self, id: u32) -> bool {
        self.bit_set.remove(id as usize)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.bit_set.is_disjoint(&other.bit_set)
    }

    pub fn is_superset(&self, other: &Self) -> bool {
        self.bit_set.is_superset(&other.bit_set)
    }

    pub fn union_with(&mut self, other: &ColumnRefSet) {
        self.bit_set.union_with(&other.bit_set)
    }

    pub fn len(&self) -> usize {
        self.bit_set.len()
    }
}

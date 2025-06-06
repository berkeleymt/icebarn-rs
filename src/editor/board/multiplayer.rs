use crdts::{orswot, CmRDT};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Board;
use crate::{
    bpz::Pos,
    editor::board::util::{PosOrd, UnorderedPair},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Op(orswot::Op<UnorderedPair<PosOrd>, Uuid>);

impl From<orswot::Op<UnorderedPair<PosOrd>, Uuid>> for Op {
    fn from(value: orswot::Op<UnorderedPair<PosOrd>, Uuid>) -> Self {
        Self(value)
    }
}

type TransmitFn = Box<dyn Fn(Op) + Send + Sync>;

pub struct MultiplayerBoard {
    inner: orswot::Orswot<UnorderedPair<PosOrd>, Uuid>,
    actor: Uuid,
    transmit_op: TransmitFn,
}

impl MultiplayerBoard {
    pub fn new(transmit_op: TransmitFn) -> Self {
        Self {
            inner: Default::default(),
            actor: Uuid::new_v4(),
            transmit_op,
        }
    }

    pub fn apply_op(&mut self, op: Op) {
        self.inner.apply(op.0);
    }
}

impl Board for MultiplayerBoard {
    fn contains(&self, p1: Pos, p2: Pos) -> bool {
        self.inner.contains(&UnorderedPair::new(p1, p2)).val
    }

    fn draw(&mut self, p1: Pos, p2: Pos) {
        let add_ctx = self.inner.read_ctx().derive_add_ctx(self.actor);
        let op = self.inner.add(UnorderedPair::new(p1, p2), add_ctx);
        self.apply_op(op.clone().into());
        (self.transmit_op)(op.into());
    }

    fn erase(&mut self, p1: Pos, p2: Pos) {
        let rm_ctx = self.inner.read_ctx().derive_rm_ctx();
        let op = self.inner.rm(UnorderedPair::new(p1, p2), rm_ctx);
        self.apply_op(op.clone().into());
        (self.transmit_op)(op.into());
    }
}

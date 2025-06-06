use crdts::{orswot, CmRDT};
use futures::channel::mpsc;
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

pub struct MultiplayerBoard {
    inner: orswot::Orswot<UnorderedPair<PosOrd>, Uuid>,
    actor: Uuid,
    tx: mpsc::Sender<Op>,
}

impl MultiplayerBoard {
    pub fn new(tx: mpsc::Sender<Op>) -> Self {
        Self {
            inner: Default::default(),
            actor: Uuid::new_v4(),
            tx,
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
        let op: Op = self.inner.add(UnorderedPair::new(p1, p2), add_ctx).into();
        self.tx.try_send(op.clone()).unwrap();
        self.apply_op(op.clone());
    }

    fn erase(&mut self, p1: Pos, p2: Pos) {
        let rm_ctx = self.inner.read_ctx().derive_rm_ctx();
        let op: Op = self.inner.rm(UnorderedPair::new(p1, p2), rm_ctx).into();
        self.tx.try_send(op.clone()).unwrap();
        self.apply_op(op.clone());
    }
}

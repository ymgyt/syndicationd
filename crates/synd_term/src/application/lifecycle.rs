use std::marker::PhantomData;

pub struct TermUninit;
pub struct TermReady;
pub struct TermRestored;

pub struct SessPending;
pub struct SessReady;

pub(super) struct Lifecycle<Term, Sess> {
    _term: PhantomData<Term>,
    _sess: PhantomData<Sess>,
}

impl<Term, Sess> Lifecycle<Term, Sess> {
    pub(super) const fn new() -> Self {
        Self {
            _term: PhantomData,
            _sess: PhantomData,
        }
    }
}

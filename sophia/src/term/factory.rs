//! A `TermFactory` can be used to create terms while preventing the proliferation of duplicate string.
//! 
//! This is especially useful for  [`RcTerm`s](../index.html) and [`ArcTerm`s](../index.html),
//! for which two implementations of `TermFactory` are provided.

use std::rc;
use std::sync;

use weak_table::{WeakHashSet};

use super::*;

pub trait TermFactory {
    type TermData: AsRef<str> + Clone + Eq + Hash;

    fn get_holder(&mut self, txt: &str) -> Self::TermData;

    fn iri<T> (&mut self, iri: T) -> Result<Term<Self::TermData>> where
        T: AsRef<str> + Clone + Eq + Hash,
    {
        Term::new_iri(self.get_holder(iri.as_ref()))
    }

    fn iri2<T, U> (&mut self, ns: T, suffix: U) -> Result<Term<Self::TermData>> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
    {
        Term::new_iri2(self.get_holder(ns.as_ref()), self.get_holder(suffix.as_ref()))
    }

    fn bnode<T> (&mut self, id: T) -> Result<Term<Self::TermData>> where
        T: AsRef<str> + Clone + Eq + Hash,
    {
        Term::new_bnode(self.get_holder(id.as_ref()))
    }

    fn literal_lang<T, U> (&mut self, txt: T, lang: U) -> Result<Term<Self::TermData>> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
    {
        Term::new_literal_lang(self.get_holder(txt.as_ref()), self.get_holder(lang.as_ref()))
    }

    fn literal_dt<T, U> (&mut self, txt: T, dt: Term<U>) -> Result<Term<Self::TermData>> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
        Self::TermData: Debug,
    {
        Term::new_literal_dt(self.get_holder(txt.as_ref()), self.copy(&dt))
    }

    fn variable<T> (&mut self, name: T) -> Result<Term<Self::TermData>> where
        T: AsRef<str> + Clone + Eq + Hash,
    {
        Term::new_variable(self.get_holder(name.as_ref()))
    }

    fn copy<T> (&mut self, other: &Term<T>) -> Term<Self::TermData> where
        T: AsRef<str> + Clone + Eq + Hash,
    {
        Term::from_with(other, |txt| self.get_holder(txt))
    }

    fn copy_normalized<T> (&mut self, other: &Term<T>, norm: Normalization) -> Term<Self::TermData> where
        T: AsRef<str> + Clone + Eq + Hash,
    {
        Term::normalized_with(other, |txt| self.get_holder(txt), norm)
    }

    fn shrink_to_fit(&mut self);
}



pub type RcTermFactory = WeakHashSet<rc::Weak<str>>;

impl TermFactory for RcTermFactory {
    type TermData = Rc<str>;

    fn get_holder(&mut self, txt: &str) -> Rc<str> {
        if let Some(holder) = self.get(txt) {
            holder
        } else {
            let holder: Rc<str> = Rc::from(txt);
            self.insert(holder.clone());
            holder
        }
    }

    fn shrink_to_fit(&mut self) {
        WeakHashSet::shrink_to_fit(self);
    }
}

pub type ArcTermFactory = WeakHashSet<sync::Weak<str>>;

impl TermFactory for ArcTermFactory {
    type TermData = sync::Arc<str>;

    fn get_holder(&mut self, txt: &str) -> sync::Arc<str> {
        if let Some(holder) = self.get(txt) {
            holder
        } else {
            let holder: sync::Arc<str> = sync::Arc::from(txt);
            self.insert(holder.clone());
            holder
        }
    }

    fn shrink_to_fit(&mut self) {
        WeakHashSet::shrink_to_fit(self);
    }
}



#[cfg(test)]
mod test {
    // Nothing really worth testing here
}
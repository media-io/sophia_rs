// this module is transparently re-exported by its parent `graph`
// It defines implementation of Graph and MutableGraph for existing types.

use std::collections::HashSet;
use std::hash::Hash;

use resiter::oks::*;

use super::*;
use crate::error::*;
use crate::term::*;
use crate::term::graph_key::*;
use crate::triple::*;
use crate::quad::*;
use crate::quad::stream::AsQuadSource;


impl<'a, Q> Dataset<'a> for [Q] where
    Q: Quad<'a>+'a,
{
    type Quad = &'a Q;
    type Error = Never;

    #[inline]
    fn quads(&'a self) -> DQuadSource<Self> {
        Box::new(
            <[Q]>::iter(self).as_quad_source()
        )
    }
}



impl<'a, Q> Dataset<'a> for Vec<Q> where
    Q: Quad<'a>+'a,
{
    type Quad = &'a Q;
    type Error = Never;

    #[inline]
    fn quads(&'a self) -> DQuadSource<Self> {
        Box::new(
            <[Q]>::iter(self).as_quad_source()
        )
    }
}

impl MutableDataset for Vec<([BoxTerm;3], GraphKey<Box<str>>)> where
{
    type MutationError = Never;

    fn insert<T, U, V, W> (&mut self, s: &Term<T>, p: &Term<U>, o: &Term<V>, g: &GraphKey<W>) -> MDResult< Self, bool> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
        V: AsRef<str> + Clone + Eq + Hash,
        W: AsRef<str> + Clone + Eq + Hash,
    {
        let s = BoxTerm::from(s);
        let p = BoxTerm::from(p);
        let o = BoxTerm::from(o);
        let g = GraphKey::from(g);
        self.push(([s, p, o], g));
        Ok(true)
    }
    fn remove<T, U, V, W> (&mut self, s: &Term<T>, p: &Term<U>, o: &Term<V>, g: &GraphKey<W>) -> MDResult< Self, bool> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
        V: AsRef<str> + Clone + Eq + Hash,
        W: AsRef<str> + Clone + Eq + Hash,
    {
        let i = self.quads().oks().position(|q|
            s == q.s() && p == q.p() && o == q.o() && g == q.g()
        );
        if let Some(i) = i {
            self.swap_remove(i);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}



impl<'a, Q> Dataset<'a> for HashSet<Q> where
    Q: Eq + Hash + Quad<'a> + 'a,
{
    type Quad = &'a Q;
    type Error = Never;

    #[inline]
    fn quads(&'a self) -> DQuadSource<Self> {
        Box::from(self.iter().as_quad_source())
    }
}

impl MutableDataset for HashSet<([BoxTerm;3], GraphKey<Box<str>>)> where
{
    type MutationError = Never;

    fn insert<T, U, V, W> (&mut self, s: &Term<T>, p: &Term<U>, o: &Term<V>, g: &GraphKey<W>) -> MDResult< Self, bool> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
        V: AsRef<str> + Clone + Eq + Hash,
        W: AsRef<str> + Clone + Eq + Hash,
    {
        let s = BoxTerm::from(s);
        let p = BoxTerm::from(p);
        let o = BoxTerm::from(o);
        let g = GraphKey::from(g);
        Ok(HashSet::insert(self, ([s, p, o], g)))
    }
    fn remove<T, U, V, W> (&mut self, s: &Term<T>, p: &Term<U>, o: &Term<V>, g: &GraphKey<W>) -> MDResult< Self, bool> where
        T: AsRef<str> + Clone + Eq + Hash,
        U: AsRef<str> + Clone + Eq + Hash,
        V: AsRef<str> + Clone + Eq + Hash,
        W: AsRef<str> + Clone + Eq + Hash,
    {
        let s = BoxTerm::from(s);
        let p = BoxTerm::from(p);
        let o = BoxTerm::from(o);
        let g = GraphKey::from(g);
        Ok(HashSet::remove(self, &([s, p, o], g)))
    }
}

impl<'a, T> SetDataset for HashSet<T> where
    T: Eq + Hash + Triple<'a> + 'a,
{}




#[cfg(test)]
mod test {
    use std::collections::HashSet;
    use resiter::oks::*;

    use crate::dataset::*;
    use crate::ns::*;
    use crate::term::{*, graph_key::*};
    use crate::quad::stream::*;

    #[test]
    fn test_slice() {
        let gn = StaticTerm::new_bnode("x").unwrap();
        let gn = GraphKey::<&str>::from(&gn);
        let d = [
            ([rdf::type_, rdf::type_, rdf::Property], GraphKey::Default),
            ([rdf::Property, rdf::type_, rdfs::Class], GraphKey::Default),
            ([rdfs::Class, rdf::type_, rdfs::Class], gn.clone()),
        ];
        let len = d.quads().oks().count();
        assert_eq!(len, 3);
        let len = d.quads_with_o(&rdfs::Class).oks().count();
        assert_eq!(len, 2);
        let len = d.quads_with_g(&gn).oks().count();
        assert_eq!(len, 1);
    }

    type VecAsDataset = Vec<([BoxTerm;3], GraphKey<Box<str>>)>;
    test_dataset_impl!(vec, VecAsDataset, false);

    type HashSetAsDataset = HashSet<([BoxTerm;3], GraphKey<Box<str>>)>;
    test_dataset_impl!(hashset, HashSetAsDataset);
}
//! Binary decision diagram representation
use crate::repr::var_label::VarSet;

use super::{
    ddnnf::*,
    model::PartialModel,
    var_label::{Literal, VarLabel},
    var_order::VarOrder,
    wmc::WmcParams,
};
use core::fmt::Debug;
use std::iter::FromIterator;

/// Core BDD pointer datatype
#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, PartialOrd, Ord)]
pub enum BddPtr {
    Compl(*mut BddNode),
    Reg(*mut BddNode),
    PtrTrue,
    PtrFalse,
}

use bit_set::BitSet;
use bumpalo::Bump;
use BddPtr::*;

impl BddPtr {
    pub fn new_reg(n: *mut BddNode) -> BddPtr {
        Reg(n)
    }

    pub fn new_compl(n: *mut BddNode) -> BddPtr {
        Compl(n)
    }

    /// Gets the varlabel of &self
    /// Panics if not a node
    pub fn var(&self) -> VarLabel {
        self.into_node().var
    }

    /// Get a mutable reference to the node that &self points to
    ///
    /// Panics if not a node pointer
    pub fn mut_node_ref(&self) -> &mut BddNode {
        unsafe {
            match &self {
                Reg(x) => &mut (**x),
                Compl(x) => &mut (**x),
                _ => panic!("Dereferencing constant in deref_or_panic"),
            }
        }
    }

    /// Dereferences the BddPtr into a BddNode
    /// The pointer is returned in regular-form (i.e., if &self is complemented, then
    /// the returned BddNode incorporates this information)
    ///
    /// Panics if the pointer is constant (i.e., true or false)
    pub fn into_node(&self) -> BddNode {
        unsafe {
            match &self {
                Reg(x) => (**x).clone(),
                Compl(x) => {
                    let BddNode {
                        var,
                        low,
                        high,
                        data,
                    } = **x;
                    BddNode {
                        var,
                        low: low.neg(),
                        high: high.neg(),
                        data,
                    }
                }
                _ => panic!("Dereferencing constant in deref_or_panic"),
            }
        }
    }

    /// convert a BddPtr into a regular (non-complemented) pointer
    pub fn to_reg(&self) -> BddPtr {
        match &self {
            Compl(x) => Reg(*x),
            Reg(x) => Reg(*x),
            PtrTrue => PtrTrue,
            PtrFalse => PtrTrue,
        }
    }

pub fn low(&self) -> BddPtr {
        unsafe {
            match &self {
                Compl(x) => (**x).low.neg(),
                Reg(x) => (**x).low,
                PtrTrue | PtrFalse => panic!("deref constant BDD"),
            }
        }
    }

    pub fn low_raw(&self) -> BddPtr {
        unsafe {
            match &self {
                Compl(x) => (**x).low,
                Reg(x) => (**x).low,
                PtrTrue | PtrFalse => panic!("deref constant BDD"),
            }
        }
    }

    pub fn high_raw(&self) -> BddPtr {
        unsafe {
            match &self {
                Compl(x) => (**x).high,
                Reg(x) => (**x).high,
                PtrTrue | PtrFalse => panic!("deref constant BDD"),
            }
        }
    }

    pub fn high(&self) -> BddPtr {
        unsafe {
            match &self {
                Compl(x) => (**x).high.neg(),
                Reg(x) => (**x).high,
                PtrTrue | PtrFalse => panic!("deref constant BDD"),
            }
        }
    }


    /// Traverses the BDD and clears all scratch memory (sets it equal to 0)
    pub fn clear_scratch(&self) -> () {
        if self.is_const() {
            return;
        } else {
            let n = self.mut_node_ref();
            if n.data != 0 {
                n.data = 0;
                n.low.clear_scratch();
                n.high.clear_scratch();
            }
        }
    }

    /// true if the BddPtr points to a constant (i.e., True or False)
    pub fn is_const(&self) -> bool {
        match &self {
            Compl(_) => false,
            Reg(_) => false,
            PtrTrue => true,
            PtrFalse => true,
        }
    }

    /// Gets the scratch value stored in `&self`
    ///
    /// Panics if not node.
    pub fn get_scratch<T>(&self) -> Option<&T> {
        unsafe {
            let ptr = self.mut_node_ref().data;
            if ptr == 0 {
                return None;
            } else {
                return Some(&*(self.into_node().data as *const T));
            }
        }
    }

    /// Set the scratch in this node to the value `v`.
    ///
    /// Panics if not a node.
    ///
    /// Invariant: values stored in `set_scratch` must not outlive
    /// the provided allocator `alloc` (i.e., calling `get_scratch`
    /// involves dereferencing a pointer stored in `alloc`)
    pub fn set_scratch<T>(&self, alloc: &mut Bump, v: T) -> () {
        self.mut_node_ref().data = (alloc.alloc(v) as *const T) as usize;
    }

    pub fn to_string_debug(&self) -> String {
        fn print_bdd_helper(ptr: BddPtr) -> String {
            if ptr.is_true() {
                return String::from("T");
            } else if ptr.is_false() {
                return String::from("F");
            } else {
                let l_p = if ptr.is_neg() {
                    ptr.low_raw().neg()
                } else {
                    ptr.low_raw()
                };
                let h_p = if ptr.is_neg() {
                    ptr.high_raw().neg()
                } else {
                    ptr.high_raw()
                };
                let l_s = print_bdd_helper(l_p);
                let h_s = print_bdd_helper(h_p);
                format!("({}, {}, {})", ptr.var().value(), h_s, l_s)
            }
        }
        print_bdd_helper(*self)
    }

    /// Print a debug form of the BDD with the label remapping given by `map`
    pub fn print_bdd_lbl(&self, ptr: BddPtr, map: &HashMap<VarLabel, VarLabel>) -> String {
        panic!("todo")
        // use crate::builder::repr::builder_bdd::PointerType::*;
        // fn print_bdd_helper(
        //     t: &BddManager,
        //     ptr: BddPtr,
        //     map: &HashMap<VarLabel, VarLabel>,
        // ) -> String {
        //     match ptr.ptr_type() {
        //         PtrTrue => String::from("T"),
        //         PtrFalse => String::from("T"),
        //         PtrNode => {
        //             let l_p = t.low(ptr);
        //             let h_p = t.high(ptr);
        //             let l_s = print_bdd_helper(t, l_p, map);
        //             let r_s = print_bdd_helper(t, h_p, map);
        //             let lbl = ptr.label();
        //             format!(
        //                 "({:?}, {}{}, {}{})",
        //                 map.get(&lbl).unwrap_or(&lbl).value(),
        //                 if l_p.is_compl() { "!" } else { "" },
        //                 l_s,
        //                 if h_p.is_compl() { "!" } else { "" },
        //                 r_s
        //             )
        //         }
        //     }
        // }
        // let s = print_bdd_helper(self, ptr, map);
        // format!("{}{}", if ptr.is_compl() { "!" } else { "" }, s)
    }

    fn bdd_fold_h<T: Clone + Copy + Debug, F: Fn(VarLabel, T, T) -> T>(
        &self,
        f: &F,
        low_v: T,
        high_v: T,
        alloc: &mut Bump
    ) -> T {
        if self.is_true() {
            return high_v;
        } else if self.is_false() {
            return low_v;
        } else {
            if self.get_scratch::<(Option<T>, Option<T>)>().is_none() {
                self.set_scratch::<(Option<T>, Option<T>)>(alloc, (None, None));
            }
            match self.get_scratch::<(Option<T>, Option<T>)>() {
                Some((Some(v), _)) if self.is_neg() => return *v,
                Some((_, Some(v))) if !self.is_neg() => return *v,
                Some((prev_low, prev_high)) => {
                    let l = self.low().bdd_fold_h(f, low_v, high_v, alloc);
                    let h = self.high().bdd_fold_h(f, low_v, high_v, alloc);
                    let res = f(self.var(), l, h);
                    if self.is_neg() {
                        self.set_scratch::<(Option<T>, Option<T>)>(alloc, (Some(res), *prev_high));
                    } else {
                        self.set_scratch::<(Option<T>, Option<T>)>(alloc, (*prev_low, Some(res)));
                    }
                    return res;
                },
                _ => panic!("unreachable")
            }
        }
    }

    pub fn bdd_fold<T: Clone + Copy + Debug, F: Fn(VarLabel, T, T) -> T>(
        &self,
        f: &F,
        low_v: T,
        high_v: T,
    ) -> T {
        let r = self.bdd_fold_h(f, low_v, high_v, &mut Bump::new());
        self.clear_scratch();
        return r;
    }


    /// evaluates a circuit on a partial marginal MAP assignment to get an upper-bound on the wmc
    /// maxes over the `map_vars`, applies the `partial_map_assgn`
    fn marginal_map_eval(
        &self,
        partial_map_assgn: &PartialModel,
        map_vars: &BitSet,
        wmc: &WmcParams<f64>,
    ) -> f64 {
        let mut v = self.bdd_fold(
            &|varlabel, low, high| {
                let (low_w, high_w) = wmc.get_var_weight(varlabel);
                match partial_map_assgn.get(varlabel) {
                    None => {
                        if map_vars.contains(varlabel.value_usize()) {
                            f64::max(*low_w * low, *high_w * high)
                        } else {
                            (*low_w * low) + (*high_w * high)
                        }
                    }
                    Some(true) => high,
                    Some(false) => low,
                }
            },
            wmc.zero,
            wmc.one,
        );
        // multiply in weights of all variables in the partial assignment
        for lit in partial_map_assgn.assignment_iter() {
            let (l, h) = wmc.get_var_weight(lit.get_label());
            if lit.get_polarity() {
                v *= h;
            } else {
                v *= l;
            }
        }
        return v;
    }

    fn marginal_map_h(
        &self,
        cur_lb: f64,
        cur_best: PartialModel,
        margvars: &[VarLabel],
        wmc: &WmcParams<f64>,
        cur_assgn: PartialModel,
    ) -> (f64, PartialModel) {
        match margvars {
            [] => {
                let margvar_bits = BitSet::new();
                let possible_best = self.marginal_map_eval(&cur_assgn, &margvar_bits, wmc);
                if possible_best > cur_lb {
                    (possible_best, cur_assgn)
                } else {
                    (cur_lb, cur_best)
                }
            }
            [x, end @ ..] => {
                let mut best_model = cur_best;
                let mut best_lb = cur_lb;
                let margvar_bits = BitSet::from_iter(end.iter().map(|x| x.value_usize()));

                let mut true_model = cur_assgn.clone();
                true_model.set(*x, true);
                let mut false_model = cur_assgn.clone();
                false_model.set(*x, false);

                let true_ub = self.marginal_map_eval(&true_model, &margvar_bits, wmc);
                let false_ub = self.marginal_map_eval(&false_model, &margvar_bits, wmc);

                // branch on the greater upper-bound first
                let order = if true_ub > false_ub {
                    [(true_ub, true_model), (false_ub, false_model)]
                } else {
                    [(false_ub, false_model), (true_ub, true_model)]
                };
                for (upper_bound, partialmodel) in order {
                    // branch + bound
                    if upper_bound > best_lb {
                        (best_lb, best_model) = self.marginal_map_h(
                            best_lb,
                            best_model,
                            end,
                            wmc,
                            partialmodel.clone(),
                        );
                    }
                }
                (best_lb, best_model)
            }
        }
    }

    /// Computes the marginal map over variables `vars` of `ptr` with evidence `evidence`
    /// I.e., computes argmax_{v in vars} \sum_{v not in vars} w(ptr /\ evidence)
    /// ```
    /// use rsdd::builder::bdd_builder::{BddManager, BddWmc};
    /// use rsdd::repr::var_label::{VarLabel, Literal};
    /// use rsdd::repr::model::PartialModel;
    /// use std::collections::HashMap;
    /// use rsdd::repr::cnf::Cnf;
    /// let cnf = Cnf::from_string(String::from("(1 || 2 || 3 || 4)"));
    /// let mut mgr = BddManager::new_default_order(cnf.num_vars());
    /// let w : HashMap<VarLabel, (f64, f64)> = (0..5).map(|x| (VarLabel::new(x), (0.3, 0.7))).collect();
    /// let wmc = BddWmc::new_with_default(0.0, 1.0, w);
    /// let evidence = mgr.true_ptr();
    /// let bdd = mgr.from_cnf(&cnf);
    /// let (p, marg_map) = mgr.marginal_map(bdd, evidence, &vec![VarLabel::new(0), VarLabel::new(1)], &wmc);
    /// let expected_model = PartialModel::from_litvec(&vec![Literal::new(VarLabel::new(0), true), Literal::new(VarLabel::new(1), true)], cnf.num_vars());
    /// let expected_prob = 0.49;
    /// assert_eq!(marg_map, expected_model);
    /// ```
    pub fn marginal_map(
        &self,
        vars: &[VarLabel],
        num_vars: usize,
        wmc: &WmcParams<f64>,
    ) -> (f64, PartialModel) {
        let mut marg_vars = BitSet::new();
        for v in vars {
            marg_vars.insert(v.value_usize());
        }

        let all_true: Vec<Literal> = vars.iter().map(|x| Literal::new(*x, true)).collect();
        let cur_assgn = PartialModel::from_litvec(&all_true, num_vars);
        let lower_bound = self.marginal_map_eval(&cur_assgn, &BitSet::new(), wmc);

        self.marginal_map_h(
            lower_bound,
            cur_assgn,
            vars,
            wmc,
            PartialModel::from_litvec(&[], num_vars),
        )
    }
}

type DDNNFCache<T> = (Option<T>, Option<T>);

impl DDNNFPtr for BddPtr {
    type Order = VarOrder;

    fn true_ptr() -> BddPtr {
        PtrTrue
    }

    fn false_ptr() -> BddPtr {
        PtrFalse
    }

    /// True is this is a complemented edge pointer
    fn is_neg(&self) -> bool {
        match &self {
            Compl(_) => true,
            Reg(_) => false,
            PtrTrue => false,
            PtrFalse => false,
        }
    }

    fn is_true(&self) -> bool {
        match &self {
            Compl(_) | Reg(_) | PtrFalse => false,
            PtrTrue => true,
        }
    }

    fn is_false(&self) -> bool {
        match &self {
            Compl(_) | Reg(_) | PtrTrue => false,
            PtrFalse => true,
        }
    }

    fn fold<T: Clone + Copy + Debug, F: Fn(DDNNF<T>) -> T>(&self, o: &VarOrder, f: F) -> T {
        fn bottomup_pass_h<T: Clone + Copy + Debug, F: Fn(DDNNF<T>) -> T>(
            ptr: BddPtr,
            f: &F,
            alloc: &mut Bump,
        ) -> T {
            match ptr {
                PtrTrue => f(DDNNF::True),
                PtrFalse => f(DDNNF::False),
                Compl(_) | Reg(_) => {
                    // inside the cache, store a (compl, non_compl) pair corresponding to the
                    // complemented and uncomplemented pass over this node
                    if ptr.get_scratch::<DDNNFCache<T>>().is_none() {
                        ptr.set_scratch::<DDNNFCache<T>>(alloc, (None, None));
                    }
                    match ptr.get_scratch::<DDNNFCache<T>>() {
                        Some((Some(v), _)) if ptr.is_neg() => return v.clone(),
                        Some((_, Some(v))) if !ptr.is_neg() => return v.clone(),
                        Some((None, cached)) | Some((cached, None)) => {
                            // no cached value found, compute it
                            let l = if ptr.is_neg() {
                                ptr.low_raw().neg()
                            } else {
                                ptr.low_raw()
                            };
                            let h = if ptr.is_neg() {
                                ptr.high_raw().neg()
                            } else {
                                ptr.high_raw()
                            };

                            let low_v = bottomup_pass_h(l, f, alloc);
                            let high_v = bottomup_pass_h(h, f, alloc);
                            let top = ptr.var();

                            let lit_high = f(DDNNF::Lit(top, true));
                            let lit_low = f(DDNNF::Lit(top, false));

                            let and_low = f(DDNNF::And(lit_low, low_v));
                            let and_high = f(DDNNF::And(lit_high, high_v));

                            // in a BDD, each decision only depends on the topvar
                            let mut varset = VarSet::new();
                            varset.insert(top);

                            let or_v = f(DDNNF::Or(and_low, and_high, varset));

                            // cache and return or_v
                            if ptr.is_neg() {
                                ptr.set_scratch::<DDNNFCache<T>>(alloc, (Some(or_v), *cached));
                            } else {
                                ptr.set_scratch::<DDNNFCache<T>>(alloc, (*cached, Some(or_v)));
                            }
                            return or_v;
                        }
                        _ => panic!("unreachable"),
                    }
                }
            }
        }

        let mut alloc = Bump::new();
        let r = bottomup_pass_h(*self, &f, &mut alloc);
        self.clear_scratch();
        r
    }

    fn count_nodes(&self) -> usize {
        fn count_h(ptr: BddPtr, alloc: &mut Bump) -> usize {
            if ptr.is_const() {
                return 0;
            } else {
                match ptr.get_scratch::<usize>() {
                    Some(_) => 0,
                    None => {
                        // found a new node
                        ptr.set_scratch::<usize>(alloc, 0);
                        let sub_l = count_h(ptr.low_raw(), alloc);
                        let sub_h = count_h(ptr.high_raw(), alloc);
                        return sub_l + sub_h + 1;
                    }
                }
            }
        }
        count_h(*self, &mut Bump::new())
    }

    fn neg(&self) -> Self {
        match &self {
            Compl(x) => Reg(*x),
            Reg(x) => Compl(*x),
            PtrTrue => PtrFalse,
            PtrFalse => PtrTrue,
        }
    }




}

/// Core BDD node storage
#[derive(Debug, Clone, Eq)]
pub struct BddNode {
    pub var: VarLabel,
    pub low: BddPtr,
    pub high: BddPtr,
    /// scratch space used for caching data during traversals; ignored during
    /// equality checking and hashing
    data: usize,
}

impl BddNode {
    pub fn new(var: VarLabel, low: BddPtr, high: BddPtr) -> BddNode {
        BddNode {
            var,
            low,
            high,
            data: 0,
        }
    }
}

impl PartialEq for BddNode {
    fn eq(&self, other: &Self) -> bool {
        self.var == other.var && self.low == other.low && self.high == other.high
    }
}

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};
impl Hash for BddNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.var.hash(state);
        self.low.hash(state);
        self.high.hash(state);
    }
}

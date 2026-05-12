//! `syn` AST visitor: applies `check_one` to every named item that can carry
//! a `///` doc comment.

use std::path::{Path, PathBuf};

use syn::visit::Visit;
use syn::{
    ImplItemConst, ImplItemFn, ImplItemType, ItemConst, ItemEnum, ItemFn, ItemMod, ItemStatic,
    ItemStruct, ItemTrait, ItemType, TraitItemConst, TraitItemFn, TraitItemType,
};

use super::{Finding, check_one};

pub(crate) struct Visitor {
    path: PathBuf,
    findings: Vec<Finding>,
}

impl Visitor {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            findings: Vec::new(),
        }
    }

    pub(crate) fn into_findings(self) -> Vec<Finding> {
        self.findings
    }

    fn push(&mut self, attrs: &[syn::Attribute], ident: &syn::Ident) {
        if let Some(f) = check_one(&self.path, attrs, ident) {
            self.findings.push(f);
        }
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_const(&mut self, node: &'ast ItemConst) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_const(self, node);
    }
    fn visit_item_static(&mut self, node: &'ast ItemStatic) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_static(self, node);
    }
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.push(&node.attrs, &node.sig.ident);
        syn::visit::visit_item_fn(self, node);
    }
    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_struct(self, node);
    }
    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_enum(self, node);
    }
    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_trait(self, node);
    }
    fn visit_item_type(&mut self, node: &'ast ItemType) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_type(self, node);
    }
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_item_mod(self, node);
    }
    fn visit_trait_item_const(&mut self, node: &'ast TraitItemConst) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_trait_item_const(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        self.push(&node.attrs, &node.sig.ident);
        syn::visit::visit_trait_item_fn(self, node);
    }
    fn visit_trait_item_type(&mut self, node: &'ast TraitItemType) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_trait_item_type(self, node);
    }
    fn visit_impl_item_const(&mut self, node: &'ast ImplItemConst) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_impl_item_const(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.push(&node.attrs, &node.sig.ident);
        syn::visit::visit_impl_item_fn(self, node);
    }
    fn visit_impl_item_type(&mut self, node: &'ast ImplItemType) {
        self.push(&node.attrs, &node.ident);
        syn::visit::visit_impl_item_type(self, node);
    }
}

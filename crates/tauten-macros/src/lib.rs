//! Procedural macros for tauten. Use them through the `tauten` crate.
//!
//! * `#[derive(Cap)]`: a capability bundle whose fields are all capabilities.
//! * `#[evidence(subject = State)]`: a fact only its module can mint.
//! * `#[effects(..)]`: a function's declared effect row.
//!
//! * `#[newtype]` / `#[refined(|v| predicate)]` on a one-field tuple struct:
//!   the struct is defined in a private module, so even code next to it can
//!   build values only through `new` / `try_new`.
//! * `#[requires(condition)]` / `#[ensures(|ret| condition)]` on a function
//!   returning `Result`: checked on every call, in every build profile; a
//!   violation is returned as an error, never a panic.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote};
use syn::visit_mut::VisitMut;
use syn::{
    Attribute, Expr, Fields, ItemFn, ItemStruct, ReturnType, Type, Visibility, parse_macro_input,
};

/// A nominal newtype whose field cannot be reached or built directly.
#[proc_macro_attribute]
pub fn newtype(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[newtype] takes no arguments; use #[refined(..)] for a predicate",
        )
        .to_compile_error()
        .into();
    }
    let item = parse_macro_input!(item as ItemStruct);
    expand_wrapper(&item, None)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// A refinement type: a newtype whose values satisfy a predicate.
#[proc_macro_attribute]
pub fn refined(args: TokenStream, item: TokenStream) -> TokenStream {
    let predicate = parse_macro_input!(args as Expr);
    let item = parse_macro_input!(item as ItemStruct);
    expand_wrapper(&item, Some(&predicate))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derives that cannot construct a value: they only copy, compare, hash or
/// print an existing one. Anything else (`Default`, `Deserialize`,
/// `Arbitrary`, ...) would create a refined value without `try_new`.
const REFINED_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "Debug",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
];

/// Attributes allowed on a `#[refined]` struct besides allowlisted derives.
const REFINED_ATTRS: &[&str] = &["doc", "must_use", "repr"];

fn check_refined_attrs(attrs: &[syn::Attribute]) -> syn::Result<()> {
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("derive") {
            let derives = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )?;
            for derive in &derives {
                let last = derive.segments.last().map(|s| s.ident.to_string());
                if !last.is_some_and(|n| REFINED_DERIVES.contains(&n.as_str())) {
                    return Err(syn::Error::new_spanned(
                        derive,
                        "this derive could construct a refined value without checking the predicate; \
                         allowed derives: Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash. \
                         Implement the trait by hand through `try_new`",
                    ));
                }
            }
        } else if !REFINED_ATTRS.iter().any(|a| path.is_ident(a)) {
            return Err(syn::Error::new_spanned(
                attr,
                "attribute not allowed on a #[refined] type: it could construct a value without \
                 checking the predicate (allowed: doc, must_use, repr, and value-preserving derives)",
            ));
        }
    }
    Ok(())
}

fn expand_wrapper(item: &ItemStruct, predicate: Option<&Expr>) -> syn::Result<Tokens> {
    let Fields::Unnamed(fields) = &item.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "expected a tuple struct with one field: `struct Name(T);`",
        ));
    };
    let mut iter = fields.unnamed.iter();
    let (Some(field), None) = (iter.next(), iter.next()) else {
        return Err(syn::Error::new_spanned(
            &item.fields,
            "expected exactly one field",
        ));
    };
    if !matches!(field.vis, Visibility::Inherited) {
        return Err(syn::Error::new_spanned(
            &field.vis,
            "the field must be private: a public field would let callers skip the constructor",
        ));
    }
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "generic newtypes are not supported yet",
        ));
    }
    if predicate.is_some() {
        check_refined_attrs(&item.attrs)?;
    }
    let name = &item.ident;
    let vis = &item.vis;
    let inner = &field.ty;
    let attrs = &item.attrs;
    let module = format_ident!("__tauten_{}", name.to_string().to_lowercase());
    let constructors = predicate.map_or_else(
        || {
            quote! {
                /// Wrap a value.
                #[must_use]
                pub const fn new(value: #inner) -> Self {
                    Self(value)
                }
            }
        },
        |predicate| {
            let text = quote!(#predicate).to_string();
            let type_name = name.to_string();
            quote! {
                /// Wrap a value if it satisfies the refinement predicate.
                ///
                /// # Errors
                /// Returns [`tauten::RefineError`] if the predicate is false.
                pub fn try_new(value: #inner) -> ::core::result::Result<Self, ::tauten::RefineError> {
                    if ::tauten::contract::holds(&value, #predicate) {
                        ::core::result::Result::Ok(Self(value))
                    } else {
                        ::core::result::Result::Err(::tauten::RefineError::new(#type_name, #text))
                    }
                }
            }
        },
    );
    let conversions = if predicate.is_some() {
        quote! {
            impl ::tauten::refine::Refined for #name {
                type Inner = #inner;
            }

            impl ::core::convert::TryFrom<#inner> for #name {
                type Error = ::tauten::RefineError;
                fn try_from(value: #inner) -> ::core::result::Result<Self, Self::Error> {
                    Self::try_new(value)
                }
            }
        }
    } else {
        quote! {}
    };
    Ok(quote! {
        #[doc(hidden)]
        mod #module {
            #[allow(unused_imports, reason = "the field type may refer to items of the parent module")]
            use super::*;

            #(#attrs)*
            pub struct #name(#inner);

            impl #name {
                #constructors

                /// Borrow the wrapped value.
                #[must_use]
                pub const fn get(&self) -> &#inner {
                    &self.0
                }

                /// Unwrap the value.
                #[must_use]
                pub fn into_inner(self) -> #inner {
                    self.0
                }
            }

            impl ::core::convert::AsRef<#inner> for #name {
                fn as_ref(&self) -> &#inner {
                    &self.0
                }
            }

            #conversions
        }
        #vis use #module::#name;
    })
}

/// Precondition, checked before the body runs.
#[proc_macro_attribute]
pub fn requires(args: TokenStream, item: TokenStream) -> TokenStream {
    contract(Contract::Requires, args, item)
}

/// Postcondition on the `Ok` value, checked after the body returns.
/// `old(expr)` snapshots a `Copy` value before the body runs.
#[proc_macro_attribute]
pub fn ensures(args: TokenStream, item: TokenStream) -> TokenStream {
    contract(Contract::Ensures, args, item)
}

#[derive(Clone, Copy)]
enum Contract {
    Requires,
    Ensures,
}

fn contract(kind: Contract, args: TokenStream, item: TokenStream) -> TokenStream {
    let first = parse_macro_input!(args as Expr);
    let item = parse_macro_input!(item as ItemFn);
    expand_contracts(kind, first, item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn contract_kind(attr: &Attribute) -> Option<Contract> {
    let last = attr.path().segments.last()?;
    if last.ident == "requires" {
        Some(Contract::Requires)
    } else if last.ident == "ensures" {
        Some(Contract::Ensures)
    } else {
        None
    }
}

/// Replaces `old(expr)` with a snapshot binding and records the snapshot.
#[derive(Default)]
struct OldSnapshots {
    taken: Vec<(syn::Ident, Expr)>,
}

impl VisitMut for OldSnapshots {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Call(call) = expr
            && let Expr::Path(path) = call.func.as_ref()
            && path.path.is_ident("old")
            && call.args.len() == 1
            && let Some(arg) = call.args.first()
        {
            let ident = format_ident!("__tauten_old_{}", self.taken.len());
            self.taken.push((ident.clone(), arg.clone()));
            *expr = syn::parse_quote!(#ident);
            return;
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

fn expand_contracts(kind: Contract, first: Expr, mut item: ItemFn) -> syn::Result<Tokens> {
    if let Some(asyncness) = item.sig.asyncness {
        return Err(syn::Error::new_spanned(
            asyncness,
            "contracts on async functions are not supported yet",
        ));
    }
    let ReturnType::Type(_, ret) = &item.sig.output else {
        return Err(syn::Error::new_spanned(
            &item.sig,
            "contracts need a function returning `Result<T, E>` with `E: From<tauten::ContractViolation>`; \
             a violation is returned as an error, never a panic",
        ));
    };
    if !returns_result(ret) {
        return Err(syn::Error::new_spanned(
            ret,
            "contracts need a `Result<T, E>` return type with `E: From<tauten::ContractViolation>`",
        ));
    }
    let ret = ret.clone();
    let mut pre = Vec::new();
    let mut post = Vec::new();
    match kind {
        Contract::Requires => pre.push(first),
        Contract::Ensures => post.push(first),
    }
    let mut kept = Vec::new();
    for attr in core::mem::take(&mut item.attrs) {
        match contract_kind(&attr) {
            Some(Contract::Requires) => pre.push(attr.parse_args::<Expr>()?),
            Some(Contract::Ensures) => post.push(attr.parse_args::<Expr>()?),
            None => kept.push(attr),
        }
    }
    item.attrs = kept;

    // Message text is taken before `old(..)` is rewritten, so a violation
    // reports `old(balance)`, not the generated snapshot binding.
    let post_texts: Vec<String> = post.iter().map(|cond| quote!(#cond).to_string()).collect();
    let mut snapshots = OldSnapshots::default();
    for expr in &mut post {
        snapshots.visit_expr_mut(expr);
    }
    let function = item.sig.ident.to_string();
    let checks_pre = pre.iter().map(|cond| {
        let text = quote!(#cond).to_string();
        quote! {
            if !(#cond) {
                return ::core::result::Result::Err(::core::convert::From::from(
                    ::tauten::ContractViolation::requires(#function, #text),
                ));
            }
        }
    });
    let takes = snapshots.taken.iter().map(|(ident, expr)| {
        quote! { let #ident = ::tauten::contract::snapshot(&(#expr)); }
    });
    let checks_post = post.iter().zip(&post_texts).map(|(cond, text)| {
        quote! {
            if !::tauten::contract::holds(__tauten_ok, #cond) {
                return ::core::result::Result::Err(::core::convert::From::from(
                    ::tauten::ContractViolation::ensures(#function, #text),
                ));
            }
        }
    });
    let attrs = &item.attrs;
    let vis = &item.vis;
    let sig = &item.sig;
    let body = &item.block;
    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            #(#checks_pre)*
            #(#takes)*
            #[allow(clippy::redundant_closure_call, reason = "the body runs in a closure so `return` stays local")]
            let __tauten_result: #ret = (|| -> #ret #body)();
            if let ::core::result::Result::Ok(__tauten_ok) = &__tauten_result {
                #(#checks_post)*
            }
            __tauten_result
        }
    })
}

/// `#[derive(Cap)]`: a capability bundle. Implements `tauten::cap::Cap` and
/// asserts, field by field, that every field is itself a capability.
#[proc_macro_derive(Cap)]
pub fn derive_cap(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    expand_cap(&item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_cap(item: &syn::DeriveInput) -> syn::Result<Tokens> {
    let syn::Data::Struct(data) = &item.data else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "#[derive(Cap)] applies to structs whose fields are all capabilities",
        ));
    };
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let checks = data.fields.iter().enumerate().map(|(index, field)| {
        let access = field.ident.as_ref().map_or_else(
            || {
                let index = syn::Index::from(index);
                quote!(#index)
            },
            |ident| quote!(#ident),
        );
        let ty = &field.ty;
        quote::quote_spanned! {syn::spanned::Spanned::span(ty)=>
            __tauten_is_cap::<#ty>(&value.#access);
        }
    });
    Ok(quote! {
        impl #impl_generics ::tauten::cap::Cap for #name #ty_generics #where_clause {}

        const _: () = {
            #[allow(dead_code, reason = "compile-time check that every field is a capability")]
            fn __tauten_fields_are_caps #impl_generics (value: &#name #ty_generics) #where_clause {
                fn __tauten_is_cap<T: ::tauten::cap::Cap + ?Sized>(_: &T) {}
                #(#checks)*
            }
        };
    })
}

/// `#[derive(Bindings)]` on a struct with named fields whose types are
/// `PartialEq`: `diff(&self, current)` names every field that differs, and
/// `revalidate(&self, current)` returns `Evidence<'_, Fresh<Self>>` about
/// `current` when no field differs, or `Stale`. A field added later is
/// compared without further edits.
#[proc_macro_derive(Bindings)]
pub fn derive_bindings(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    expand_bindings(&item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_bindings(item: &syn::DeriveInput) -> syn::Result<Tokens> {
    let syn::Data::Struct(data) = &item.data else {
        return Err(syn::Error::new_spanned(
            item,
            "#[derive(Bindings)] needs a struct with named fields",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "#[derive(Bindings)] needs a struct with named fields",
        ));
    };
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let idents: Vec<_> = fields
        .named
        .iter()
        .filter_map(|f| f.ident.as_ref())
        .collect();
    let labels = idents.iter().map(|i| i.to_string());
    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// The fields whose value differs from `current`, in
            /// declaration order (generated by `#[derive(Bindings)]`).
            #[must_use]
            pub fn diff(&self, current: &Self) -> ::tauten::__private::Vec<&'static str> {
                let mut changed = ::tauten::__private::Vec::new();
                #(
                    if self.#idents != current.#idents {
                        changed.push(#labels);
                    }
                )*
                changed
            }

            /// Evidence that `current` still has every value recorded in
            /// `self` (generated by `#[derive(Bindings)]`).
            ///
            /// # Errors
            /// `Stale` when any field differs.
            pub fn revalidate<'s>(
                &self,
                current: &'s Self,
            ) -> ::core::result::Result<
                ::tauten::origin::Evidence<'s, ::tauten::origin::Fresh<Self>>,
                ::tauten::origin::Stale,
            > {
                if true #(&& self.#idents == current.#idents)* {
                    ::core::result::Result::Ok(::tauten::origin::Evidence::__mint(
                        ::tauten::origin::Fresh::__new(),
                        current,
                    ))
                } else {
                    ::core::result::Result::Err(::tauten::origin::Stale)
                }
            }
        }
    })
}

/// Derives a fact may carry: none of them can build a value.
const EVIDENCE_DERIVES: &[&str] = &["Debug", "PartialEq", "Eq"];

/// `#[evidence(subject = State)]` on a struct with named fields: a fact
/// about `State` that only the declaring module can create, as
/// `tauten::origin::Evidence<'_, Self>`, through the generated `mint`.
#[proc_macro_attribute]
pub fn evidence(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated);
    let item = parse_macro_input!(item as ItemStruct);
    expand_evidence(&args, &item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// `#[evidence(subject = T)]`, optionally with `exclusive` (a `Permit`
/// holding `&mut T`) and `stamped` (a `Stamped` fact checked against
/// `T: Versioned` before use).
struct EvidenceArgs<'a> {
    subject: &'a syn::Expr,
    exclusive: bool,
    stamped: bool,
}

fn evidence_args(
    args: &syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>,
) -> syn::Result<EvidenceArgs<'_>> {
    let usage = "expected `#[evidence(subject = StateType)]`, optionally followed by `exclusive` and/or `stamped`";
    let mut subject = None;
    let mut exclusive = false;
    let mut stamped = false;
    for meta in args {
        match meta {
            syn::Meta::NameValue(nv) if nv.path.is_ident("subject") && subject.is_none() => {
                subject = Some(&nv.value);
            }
            syn::Meta::Path(p) if p.is_ident("exclusive") && !exclusive => exclusive = true,
            syn::Meta::Path(p) if p.is_ident("stamped") && !stamped => stamped = true,
            syn::Meta::Path(_) | syn::Meta::List(_) | syn::Meta::NameValue(_) => {
                return Err(syn::Error::new_spanned(meta, usage));
            }
        }
    }
    let Some(subject) = subject else {
        return Err(syn::Error::new_spanned(args, usage));
    };
    Ok(EvidenceArgs {
        subject,
        exclusive,
        stamped,
    })
}

fn expand_evidence(
    args: &syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>,
    item: &ItemStruct,
) -> syn::Result<Tokens> {
    let EvidenceArgs {
        subject,
        exclusive,
        stamped,
    } = evidence_args(args)?;
    let Fields::Named(fields) = &item.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "#[evidence] needs a struct with named fields (they become the fact's parameters)",
        ));
    };
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "generic evidence types are not supported yet",
        ));
    }
    for field in &fields.named {
        if !matches!(field.vis, Visibility::Inherited) {
            return Err(syn::Error::new_spanned(
                &field.vis,
                "evidence fields must be private: a public field would let callers forge the fact",
            ));
        }
    }
    for attr in &item.attrs {
        let path = attr.path();
        if path.is_ident("derive") {
            let derives = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )?;
            for derive in &derives {
                let last = derive.segments.last().map(|s| s.ident.to_string());
                if !last.is_some_and(|n| EVIDENCE_DERIVES.contains(&n.as_str())) {
                    return Err(syn::Error::new_spanned(
                        derive,
                        "evidence may derive only Debug, PartialEq and Eq: one check, one use \
                         (no Clone/Copy) and no construction outside `mint` (no Default/Deserialize)",
                    ));
                }
            }
        } else if !path.is_ident("doc") {
            return Err(syn::Error::new_spanned(
                attr,
                "attribute not allowed on an #[evidence] type (allowed: doc, and derives of Debug, PartialEq, Eq)",
            ));
        }
    }
    let name = &item.ident;
    let vis = &item.vis;
    let attrs = &item.attrs;
    let module = format_ident!("__tauten_evidence_{}", name.to_string().to_lowercase());
    let field_defs = fields.named.iter().map(|f| {
        let ident = &f.ident;
        let ty = &f.ty;
        let fattrs = &f.attrs;
        quote!(#(#fattrs)* #ident: #ty)
    });
    let params = fields.named.iter().map(|f| {
        let ident = &f.ident;
        let ty = &f.ty;
        quote!(#ident: #ty)
    });
    let inits: Vec<_> = fields.named.iter().map(|f| f.ident.as_ref()).collect();
    let param_list: Vec<_> = params.collect();
    let exclusive_mint = exclusive.then(|| {
        quote! {
            /// Record that the fact holds for `subject`, keeping the only
            /// mutable borrow of it until the permit is acted on.
            pub(super) const fn mint_exclusive(
                subject: &mut #subject,
                #(#param_list,)*
            ) -> ::tauten::origin::Permit<'_, Self> {
                ::tauten::origin::Permit::__mint(Self { #(#inits,)* }, subject)
            }
        }
    });
    let stamped_mint = stamped.then(|| {
        quote! {
            /// Record that the fact holds for `subject` at its current
            /// version; verify it against the subject before acting.
            pub(super) fn mint_stamped(
                subject: &#subject,
                #(#param_list,)*
            ) -> ::tauten::origin::Stamped<Self> {
                ::tauten::origin::Stamped::__mint(
                    Self { #(#inits,)* },
                    <#subject as ::tauten::origin::Versioned>::version(subject),
                )
            }
        }
    });
    let getters = fields.named.iter().map(|f| {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! {
            #[doc = concat!("The `", stringify!(#ident), "` the fact was checked for.")]
            #[must_use]
            pub const fn #ident(&self) -> &#ty {
                &self.#ident
            }
        }
    });
    Ok(quote! {
        #[doc(hidden)]
        mod #module {
            #[allow(unused_imports, reason = "field and subject types may refer to the parent module")]
            use super::*;

            #(#attrs)*
            pub struct #name {
                #(#field_defs,)*
            }

            impl ::tauten::origin::Proposition for #name {
                type Subject = #subject;
            }

            impl #name {
                /// Record that the fact holds for `subject`. Callable only
                /// from the module that declares the fact, where its check
                /// lives.
                pub(super) const fn mint(
                    subject: &#subject,
                    #(#param_list,)*
                ) -> ::tauten::origin::Evidence<'_, Self> {
                    ::tauten::origin::Evidence::__mint(Self { #(#inits,)* }, subject)
                }

                #exclusive_mint
                #stamped_mint

                #(#getters)*
            }
        }
        #vis use #module::#name;
    })
}

/// `#[effects(label, ...)]` on a function: its effect row. The labels are
/// validated here and listed in the function's docs; `cargo tauten check`
/// compares them with the signature (TAU-F001).
#[proc_macro_attribute]
pub fn effects(args: TokenStream, item: TokenStream) -> TokenStream {
    let labels = parse_macro_input!(args with syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated);
    let item = parse_macro_input!(item as ItemFn);
    expand_effects(&labels, item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Kept in step with `tauten::effects::LABELS`.
const EFFECT_LABELS: &[&str] = &[
    "clock", "env", "fs_read", "fs_write", "net", "fail", "suspend",
];

fn expand_effects(
    labels: &syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>,
    item: ItemFn,
) -> syn::Result<Tokens> {
    let mut seen: Vec<String> = Vec::new();
    for label in labels {
        let text = label.to_string();
        if !EFFECT_LABELS.contains(&text.as_str()) {
            return Err(syn::Error::new_spanned(
                label,
                format!(
                    "unknown effect `{text}`; known effects: {}",
                    EFFECT_LABELS.join(", ")
                ),
            ));
        }
        if seen.contains(&text) {
            return Err(syn::Error::new_spanned(
                label,
                format!("effect `{text}` listed twice"),
            ));
        }
        seen.push(text);
    }
    let row = if seen.is_empty() {
        "none".to_owned()
    } else {
        seen.join(", ")
    };
    let doc = format!("Effects: `{{{row}}}`");
    let mut item = item;
    item.attrs.push(syn::parse_quote!(#[doc = ""]));
    item.attrs.push(syn::parse_quote!(#[doc = #doc]));
    Ok(quote!(#item))
}

fn returns_result(ty: &Type) -> bool {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "syn::Type is non_exhaustive; every other type is not a Result"
    )]
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "Result"),
        Type::Paren(inner) => returns_result(&inner.elem),
        Type::Group(inner) => returns_result(&inner.elem),
        _ => false,
    }
}

extern crate proc_macro;
use darling::{FromField, FromDeriveInput, ast, ToTokens};

use quote::quote;
use syn::{parse_macro_input};

#[derive(Debug, Clone, FromField)]
#[darling(attributes(attribute))]
struct Attribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    ignore: bool,
}

#[derive(Debug, FromDeriveInput)]
struct VertexInput {
    ident: syn::Ident,
    data: ast::Data<(), Attribute>,
}

#[proc_macro_derive(VertexAttributes)]
pub fn derive_vertex_attributes(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item);
    let input = VertexInput::from_derive_input(&input).expect("Cannot parse non-derive input");
    let attributes = input.data.as_ref().take_struct().expect("Cannot be an enum").fields.into_iter().cloned().filter(|attr| !attr.ignore).collect::<Vec<_>>();

    let ident = input.ident;

    let input = attributes.iter().enumerate().map(|(_i, attr)| {
        let ty = &attr.ty;
        let ident = attr.ident.clone().expect("Tuple structs are not supported");
        quote!(::violette::vertex::VertexDesc::from_gl_type::<#ty>(::bytemuck::offset_of!(Self, #ident)))
    }).collect::<Vec<_>>();

    quote!(
        impl ::violette::vertex::VertexAttributes for #ident {
            fn attributes() -> &'static [::violette::vertex::VertexDesc] {
                vec![#(#input),*].leak()
            }
        }
    ).into()
}
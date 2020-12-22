use ast::edit::IndentLevel;
use ide_db::base_db::AnchoredPathBuf;
use syntax::{
    ast::{self, edit::AstNodeEdit, NameOwner},
    AstNode,
};

use crate::{AssistContext, AssistId, AssistKind, Assists};

// Assist: extract_module_to_file
//
// This assist extract module to file.
//
// ```
// mod foo {<|>
//     fn t() {}
// }
// ```
// ->
// ```
// mod foo;
// ```
pub(crate) fn extract_module_to_file(acc: &mut Assists, ctx: &AssistContext) -> Option<()> {
    let module_ast = ctx.find_node_at_offset::<ast::Module>()?;
    let module_name = module_ast.name()?;

    let module_def = ctx.sema.to_def(&module_ast)?;
    let parent_module = module_def.parent(ctx.db())?;

    let module_items = module_ast.item_list()?;
    let target = module_ast.syntax().text_range();
    let anchor_file_id = ctx.frange.file_id;

    acc.add(
        AssistId("extract_module_to_file", AssistKind::RefactorExtract),
        "Extract module to file",
        target,
        |builder| {
            let path = {
                let dir = match parent_module.name(ctx.db()) {
                    Some(name) if !parent_module.is_mod_rs(ctx.db()) => format!("{}/", name),
                    _ => String::new(),
                };
                format!("./{}{}.rs", dir, module_name)
            };
            let contents = {
                let items = module_items.dedent(IndentLevel(1)).to_string();
                let mut items =
                    items.trim_start_matches('{').trim_end_matches('}').trim().to_string();
                if !items.is_empty() {
                    items.push('\n');
                }
                items
            };

            builder.replace(target, format!("mod {};", module_name));

            let dst = AnchoredPathBuf { anchor: anchor_file_id, path };
            builder.create_file(dst, contents);
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::tests::check_assist;

    use super::*;

    #[test]
    fn extract_from_root() {
        check_assist(
            extract_module_to_file,
            r#"
mod tests {<|>
    #[test] fn t() {}
}
"#,
            r#"
//- /main.rs
mod tests;
//- /tests.rs
#[test] fn t() {}
"#,
        );
    }

    #[test]
    fn extract_from_submodule() {
        check_assist(
            extract_module_to_file,
            r#"
//- /main.rs
mod submodule;
//- /submodule.rs
mod inner<|> {
    fn f() {}
}
fn g() {}
"#,
            r#"
//- /submodule.rs
mod inner;
fn g() {}
//- /submodule/inner.rs
fn f() {}
"#,
        );
    }

    #[test]
    fn extract_from_mod_rs() {
        check_assist(
            extract_module_to_file,
            r#"
//- /main.rs
mod submodule;
//- /submodule/mod.rs
mod inner<|> {
    fn f() {}
}
fn g() {}
"#,
            r#"
//- /submodule/mod.rs
mod inner;
fn g() {}
//- /submodule/inner.rs
fn f() {}
"#,
        );
    }
}

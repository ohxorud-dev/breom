use super::analysis::{ast_span_to_range, offset_to_position, position_to_offset};
use crate::ast::{common::*, declarations::*, expressions::*, program::*, statements::*, types::*};
use tower_lsp::lsp_types::*;

pub struct ReferenceFinder<'a> {
    content: &'a str,
    program: &'a Program,
}

impl<'a> ReferenceFinder<'a> {
    pub fn new(content: &'a str, program: &'a Program) -> Self {
        Self { content, program }
    }

    pub fn collect_visible_symbols(&self, position: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for item in &self.program.items {
            match item {
                TopLevelItem::Function(f) => {
                    items.push(self.create_completion_item(
                        &f.name,
                        CompletionItemKind::FUNCTION,
                        "fn",
                    ));
                }
                TopLevelItem::Struct(s) => {
                    let mut detail = "struct".to_string();
                    if !s.generic_params.is_empty() {
                        detail.push_str("<...>");
                    }
                    items.push(self.create_completion_item(
                        &s.name,
                        CompletionItemKind::STRUCT,
                        &detail,
                    ));
                }
                TopLevelItem::Interface(i) => {
                    items.push(self.create_completion_item(
                        &i.name,
                        CompletionItemKind::INTERFACE,
                        "interface",
                    ));
                }
                TopLevelItem::Define(d) => {
                    items.push(self.create_completion_item(
                        &d.name,
                        CompletionItemKind::CONSTANT,
                        "const",
                    ));
                }
                _ => {}
            }
        }

        if let Some(func) = self.find_enclosing_function(position) {
            for param in &func.params {
                items.push(self.create_completion_item(
                    &param.name,
                    CompletionItemKind::VARIABLE,
                    "param",
                ));
            }
            for gp in &func.generic_params {
                items.push(self.create_completion_item(
                    &gp.name,
                    CompletionItemKind::TYPE_PARAMETER,
                    "generic",
                ));
            }

            self.collect_vars_in_block(&func.body, position, &mut items);
        }

        items
    }

    pub fn collect_defines(&self) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        for item in &self.program.items {
            if let TopLevelItem::Define(d) = item {
                items.push(self.create_completion_item(
                    &d.name,
                    CompletionItemKind::CONSTANT,
                    "const",
                ));
            }
        }
        items
    }

    pub fn collect_dot_completions(&self, position: Position) -> Vec<CompletionItem> {
        if position.character < 2 {
            return vec![];
        }

        let before_dot_pos = Position {
            line: position.line,
            character: position.character - 2,
        };

        let (var_name, _range) = match self.get_word_at(before_dot_pos) {
            Some(w) => w,
            None => {
                return vec![];
            }
        };

        let type_name = match self.resolve_type_of_var(&var_name, position) {
            Some(t) => t,
            None => return vec![],
        };

        let struct_decl = match self.find_struct_by_name(&type_name) {
            Some(s) => s,
            None => return vec![],
        };

        let mut items = Vec::new();
        let is_internal = self.is_inside_impl_of(&type_name, position);

        for member in &struct_decl.members {
            if let StructMember::Field(f) = member {
                if f.visibility == Visibility::Public || is_internal {
                    items.push(self.create_completion_item(
                        &f.name,
                        CompletionItemKind::FIELD,
                        &type_name,
                    ));
                }
            } else if let StructMember::Method(m) = member {
                if m.visibility == Visibility::Public || is_internal {
                    items.push(self.create_completion_item(
                        &m.name,
                        CompletionItemKind::METHOD,
                        &type_name,
                    ));
                }
            }
        }

        items
    }

    fn resolve_type_of_var(&self, var_name: &str, position: Position) -> Option<String> {
        if let Some(func) = self.find_enclosing_function(position) {
            for param in &func.params {
                if param.name == var_name {
                    return self.extract_type_name(&param.type_expr);
                }
            }

            if let Some(ty) = self.find_var_type_in_block(&func.body, var_name, position) {
                return Some(ty);
            }
        }

        None
    }

    fn extract_type_name(&self, type_expr: &TypeExpr) -> Option<String> {
        match type_expr {
            TypeExpr::Base(b) => Some(b.name.clone()),

            _ => None,
        }
    }

    fn find_var_type_in_block(
        &self,
        block: &Block,
        name: &str,
        limit_pos: Position,
    ) -> Option<String> {
        let limit_offset = position_to_offset(self.content, limit_pos);
        for stmt in &block.statements {
            match stmt {
                Statement::VarDecl(var) => {
                    if var.name == name && var.span.start < limit_offset {
                        if let Some(ty) = &var.type_annotation {
                            return self.extract_type_name(ty);
                        }
                    }
                }

                Statement::If(if_stmt) => {
                    if if_stmt.span.start <= limit_offset
                        && limit_offset <= if_stmt.span.end
                        && if_stmt.then_block.span.start <= limit_offset
                        && limit_offset <= if_stmt.then_block.span.end
                    {
                        if let Some(t) =
                            self.find_var_type_in_block(&if_stmt.then_block, name, limit_pos)
                        {
                            return Some(t);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_struct_by_name(&self, name: &str) -> Option<&'a StructDecl> {
        for item in &self.program.items {
            if let TopLevelItem::Struct(s) = item {
                if s.name == name {
                    return Some(s);
                }
            }
        }
        None
    }

    fn is_inside_impl_of(&self, struct_name: &str, position: Position) -> bool {
        let offset = position_to_offset(self.content, position);
        if let Some(s) = self.find_struct_by_name(struct_name) {
            if s.span.start <= offset && offset <= s.span.end {
                return true;
            }
        }
        false
    }

    fn create_completion_item(
        &self,
        label: &str,
        kind: CompletionItemKind,
        detail: &str,
    ) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        }
    }

    fn collect_vars_in_block(
        &self,
        block: &Block,
        limit_pos: Position,
        items: &mut Vec<CompletionItem>,
    ) {
        let limit_offset = position_to_offset(self.content, limit_pos);

        for stmt in &block.statements {
            match stmt {
                Statement::VarDecl(var) => {
                    if var.span.start < limit_offset {
                        let detail = if var.mutable { "mut var" } else { "var" };
                        items.push(self.create_completion_item(
                            &var.name,
                            CompletionItemKind::VARIABLE,
                            detail,
                        ));
                    }
                }
                Statement::If(if_stmt) => {
                    if if_stmt.span.start <= limit_offset && limit_offset <= if_stmt.span.end {
                        if if_stmt.then_block.span.start <= limit_offset
                            && limit_offset <= if_stmt.then_block.span.end
                        {
                            self.collect_vars_in_block(&if_stmt.then_block, limit_pos, items);
                        }
                        if let Some(else_clause) = &if_stmt.else_clause {
                            match else_clause {
                                ElseClause::Else(block) => {
                                    if block.span.start <= limit_offset
                                        && limit_offset <= block.span.end
                                    {
                                        self.collect_vars_in_block(block, limit_pos, items);
                                    }
                                }
                                ElseClause::ElseIf(_) => {}
                            }
                        }
                    }
                }
                Statement::For(for_stmt) => match for_stmt {
                    ForStmt::Condition(_, block, span) => {
                        if span.start <= limit_offset && limit_offset <= span.end {
                            self.collect_vars_in_block(block, limit_pos, items);
                        }
                    }
                    ForStmt::Infinite(block, span) => {
                        if span.start <= limit_offset && limit_offset <= span.end {
                            self.collect_vars_in_block(block, limit_pos, items);
                        }
                    }
                    ForStmt::Count(_, block, span) => {
                        if span.start <= limit_offset && limit_offset <= span.end {
                            self.collect_vars_in_block(block, limit_pos, items);
                        }
                    }
                    ForStmt::Range(range) => {
                        if range.span.start <= limit_offset && limit_offset <= range.span.end {
                            items.push(self.create_completion_item(
                                &range.index_var,
                                CompletionItemKind::VARIABLE,
                                "index",
                            ));
                            if let Some(value_var) = &range.value_var {
                                items.push(self.create_completion_item(
                                    value_var,
                                    CompletionItemKind::VARIABLE,
                                    "value",
                                ));
                            }
                            self.collect_vars_in_block(&range.body, limit_pos, items);
                        }
                    }
                },
                Statement::Match(match_stmt) => {
                    if match_stmt.span.start <= limit_offset && limit_offset <= match_stmt.span.end
                    {
                        for arm in &match_stmt.arms {
                            if arm.span.start <= limit_offset && limit_offset <= arm.span.end {
                                if let crate::ast::statements::Pattern::Binding(binding, _) =
                                    &arm.pattern
                                {
                                    items.push(self.create_completion_item(
                                        binding,
                                        CompletionItemKind::VARIABLE,
                                        "binding",
                                    ));
                                }
                                if let MatchArmBody::Block(block) = &arm.body {
                                    self.collect_vars_in_block(block, limit_pos, items);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn find_definition(&self, position: Position) -> Option<Location> {
        let (word, _range) = self.get_word_at(position)?;

        if let Some(func) = self.find_enclosing_function(position) {
            for param in &func.params {
                if param.name == word {
                    return Some(self.to_named_location(&param.span, &param.name));
                }
            }

            for gp in &func.generic_params {
                if gp.name == word {
                    return Some(self.to_named_location(&gp.span, &gp.name));
                }
            }

            if let Some(loc) = self.find_var_in_block(&func.body, &word, position) {
                return Some(loc);
            }
        }

        for item in &self.program.items {
            match item {
                TopLevelItem::Function(f) => {
                    if f.name == word {
                        return Some(self.to_named_location(&f.span, &f.name));
                    }
                }
                TopLevelItem::Struct(s) => {
                    if s.name == word {
                        return Some(self.to_named_location(&s.span, &s.name));
                    }
                }
                TopLevelItem::Interface(i) => {
                    if i.name == word {
                        return Some(self.to_named_location(&i.span, &i.name));
                    }
                }
                TopLevelItem::Define(d) => {
                    if d.name == word {
                        return Some(self.to_named_location(&d.span, &d.name));
                    }
                }
                _ => {}
            }
        }

        None
    }

    pub fn find_references(&self, position: Position) -> Vec<Location> {
        let (word, _) = match self.get_word_at(position) {
            Some(w) => w,
            None => return vec![],
        };

        if let Some(func) = self.find_enclosing_function(position) {
            let is_local_def = func.params.iter().any(|p| p.name == word)
                || self.is_var_defined_in_block(&func.body, &word);

            if is_local_def {
                let mut refs = Vec::new();
                self.collect_refs_in_function(func, &word, &mut refs);
                return refs;
            }
        }

        let mut refs = Vec::new();
        for item in &self.program.items {
            if let TopLevelItem::Function(f) = item {
                self.collect_refs_in_function(f, &word, &mut refs);
            }
        }

        for item in &self.program.items {
            match item {
                TopLevelItem::Function(f) if f.name == word => {
                    refs.push(self.to_named_location(&f.span, &f.name))
                }
                TopLevelItem::Struct(s) if s.name == word => {
                    refs.push(self.to_named_location(&s.span, &s.name))
                }
                TopLevelItem::Interface(i) if i.name == word => {
                    refs.push(self.to_named_location(&i.span, &i.name))
                }
                TopLevelItem::Define(d) if d.name == word => {
                    refs.push(self.to_named_location(&d.span, &d.name))
                }
                _ => {}
            }
        }

        refs
    }

    pub fn find_references_by_name(&self, name: &str) -> Vec<Location> {
        let mut refs = Vec::new();

        for item in &self.program.items {
            if let TopLevelItem::Function(f) = item {
                self.collect_refs_in_function(f, name, &mut refs);
            }
        }

        for item in &self.program.items {
            match item {
                TopLevelItem::Function(f) if f.name == name => {
                    refs.push(self.to_named_location(&f.span, &f.name))
                }
                TopLevelItem::Struct(s) if s.name == name => {
                    refs.push(self.to_named_location(&s.span, &s.name))
                }
                TopLevelItem::Interface(i) if i.name == name => {
                    refs.push(self.to_named_location(&i.span, &i.name))
                }
                TopLevelItem::Define(d) if d.name == name => {
                    refs.push(self.to_named_location(&d.span, &d.name))
                }
                _ => {}
            }
        }

        refs
    }

    fn get_word_at(&self, position: Position) -> Option<(String, Range)> {
        super::analysis::get_word_at_position(self.content, position)
    }

    fn to_location(&self, span: &Span) -> Location {
        Location {
            uri: Url::parse("file:///TODO").unwrap(),
            range: ast_span_to_range(self.content, span),
        }
    }

    fn to_named_location(&self, span: &Span, name: &str) -> Location {
        let range = self
            .find_name_range_in_span(span, name)
            .unwrap_or_else(|| ast_span_to_range(self.content, span));
        Location {
            uri: Url::parse("file:///TODO").unwrap(),
            range,
        }
    }

    fn find_name_range_in_span(&self, span: &Span, name: &str) -> Option<Range> {
        if name.is_empty() {
            return None;
        }

        let bytes = self.content.as_bytes();
        let start = span.start.min(bytes.len());
        let end = span.end.min(bytes.len());
        if start >= end {
            return None;
        }

        let slice = &self.content[start..end];
        let mut search_from = 0usize;
        while let Some(rel_idx) = slice[search_from..].find(name) {
            let idx = search_from + rel_idx;
            let abs_start = start + idx;
            let abs_end = abs_start + name.len();

            let prev_ok =
                abs_start == 0 || !is_ident_byte(bytes.get(abs_start - 1).copied().unwrap_or(b' '));
            let next_ok = abs_end >= bytes.len()
                || !is_ident_byte(bytes.get(abs_end).copied().unwrap_or(b' '));

            if prev_ok && next_ok {
                return Some(Range {
                    start: offset_to_position(self.content, abs_start),
                    end: offset_to_position(self.content, abs_end),
                });
            }

            search_from = idx + name.len();
            if search_from >= slice.len() {
                break;
            }
        }

        None
    }

    fn find_enclosing_function(&self, position: Position) -> Option<&'a FunctionDecl> {
        let offset = position_to_offset(self.content, position);

        for item in &self.program.items {
            if let TopLevelItem::Function(f) = item {
                if f.span.start <= offset && offset <= f.span.end {
                    return Some(f);
                }
            }
        }
        None
    }

    fn find_var_in_block(
        &self,
        block: &'a Block,
        name: &str,
        limit_pos: Position,
    ) -> Option<Location> {
        let limit_offset = position_to_offset(self.content, limit_pos);

        for stmt in &block.statements {
            match stmt {
                Statement::VarDecl(var) => {
                    if var.name == name && var.span.start < limit_offset {
                        return Some(self.to_named_location(&var.span, &var.name));
                    }
                }
                Statement::If(if_stmt) => {
                    if if_stmt.span.start <= limit_offset && limit_offset <= if_stmt.span.end {
                        if let Some(loc) =
                            self.find_var_in_block(&if_stmt.then_block, name, limit_pos)
                        {
                            return Some(loc);
                        }
                        if let Some(else_clause) = &if_stmt.else_clause {
                            match else_clause {
                                ElseClause::Else(block) => {
                                    if let Some(loc) =
                                        self.find_var_in_block(block, name, limit_pos)
                                    {
                                        return Some(loc);
                                    }
                                }
                                ElseClause::ElseIf(_) => {}
                            }
                        }
                    }
                }
                Statement::For(ForStmt::Range(range)) => {
                    if (range.index_var == name || range.value_var.as_deref() == Some(name))
                        && range.span.start <= limit_offset
                    {
                        return Some(self.to_named_location(&range.span, name));
                    }
                    if let Some(loc) = self.find_var_in_block(&range.body, name, limit_pos) {
                        return Some(loc);
                    }
                }

                _ => {}
            }
        }
        None
    }

    fn is_var_defined_in_block(&self, block: &Block, name: &str) -> bool {
        for stmt in &block.statements {
            match stmt {
                Statement::VarDecl(var) => {
                    if var.name == name {
                        return true;
                    }
                }
                Statement::If(if_stmt) => {
                    if self.is_var_defined_in_block(&if_stmt.then_block, name) {
                        return true;
                    }
                    if let Some(else_clause) = &if_stmt.else_clause {
                        match else_clause {
                            ElseClause::Else(block) => {
                                if self.is_var_defined_in_block(block, name) {
                                    return true;
                                }
                            }
                            ElseClause::ElseIf(else_if) => {
                                if self.is_var_defined_in_block(&else_if.then_block, name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                Statement::For(ForStmt::Range(range)) => {
                    if range.index_var == name || range.value_var.as_deref() == Some(name) {
                        return true;
                    }
                    if self.is_var_defined_in_block(&range.body, name) {
                        return true;
                    }
                }
                Statement::For(ForStmt::Condition(_, block, _))
                | Statement::For(ForStmt::Infinite(block, _))
                | Statement::For(ForStmt::Count(_, block, _)) => {
                    if self.is_var_defined_in_block(block, name) {
                        return true;
                    }
                }
                Statement::Match(match_stmt) => {
                    for arm in &match_stmt.arms {
                        if let crate::ast::statements::Pattern::Binding(binding, _) = &arm.pattern {
                            if binding == name {
                                return true;
                            }
                        }
                        if let MatchArmBody::Block(block) = &arm.body {
                            if self.is_var_defined_in_block(block, name) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn collect_refs_in_function(&self, func: &FunctionDecl, name: &str, refs: &mut Vec<Location>) {
        if func.name == name {
            refs.push(self.to_named_location(&func.span, &func.name));
        }

        for param in &func.params {
            if param.name == name {
                refs.push(self.to_named_location(&param.span, &param.name));
            }

            self.collect_refs_in_type(&param.type_expr, name, refs);
        }

        self.collect_refs_in_block(&func.body, name, refs);
    }

    fn collect_refs_in_block(&self, block: &Block, name: &str, refs: &mut Vec<Location>) {
        for stmt in &block.statements {
            match stmt {
                Statement::VarDecl(v) => {
                    if v.name == name {
                        refs.push(self.to_named_location(&v.span, &v.name));
                    }
                    self.collect_refs_in_expr(&v.value, name, refs);
                    if let Some(ty) = &v.type_annotation {
                        self.collect_refs_in_type(ty, name, refs);
                    }
                }
                Statement::Assignment(a) => {
                    if a.target.base == name {
                        refs.push(self.to_named_location(&a.target.span, &a.target.base));
                    }
                    self.collect_refs_in_expr(&a.value, name, refs);
                }
                Statement::Expression(e) => self.collect_refs_in_expr(e, name, refs),
                Statement::Return(r) => {
                    if let Some(e) = &r.value {
                        self.collect_refs_in_expr(e, name, refs);
                    }
                }
                Statement::Throw(e, _) => self.collect_refs_in_expr(e, name, refs),
                Statement::If(i) => {
                    self.collect_refs_in_expr(&i.condition, name, refs);
                    self.collect_refs_in_block(&i.then_block, name, refs);
                    if let Some(else_clause) = &i.else_clause {
                        match else_clause {
                            ElseClause::Else(block) => {
                                self.collect_refs_in_block(block, name, refs);
                            }
                            ElseClause::ElseIf(else_if) => {
                                self.collect_refs_in_expr(&else_if.condition, name, refs);
                                self.collect_refs_in_block(&else_if.then_block, name, refs);
                            }
                        }
                    }
                }
                Statement::For(f) => match f {
                    ForStmt::Condition(expr, block, _) => {
                        self.collect_refs_in_expr(expr, name, refs);
                        self.collect_refs_in_block(block, name, refs);
                    }
                    ForStmt::Infinite(block, _) => {
                        self.collect_refs_in_block(block, name, refs);
                    }
                    ForStmt::Count(_, block, _) => {
                        self.collect_refs_in_block(block, name, refs);
                    }
                    ForStmt::Range(r) => {
                        self.collect_refs_in_expr(&r.iterable, name, refs);
                        if r.index_var == name || r.value_var.as_deref() == Some(name) {
                            refs.push(self.to_named_location(&r.span, name));
                        }
                        self.collect_refs_in_block(&r.body, name, refs);
                    }
                },
                Statement::Match(match_stmt) => {
                    self.collect_refs_in_expr(&match_stmt.expr, name, refs);
                    for arm in &match_stmt.arms {
                        self.collect_refs_in_pattern(&arm.pattern, name, refs);
                        match &arm.body {
                            MatchArmBody::Block(block) => {
                                self.collect_refs_in_block(block, name, refs)
                            }
                            MatchArmBody::Expression(expr) => {
                                self.collect_refs_in_expr(expr, name, refs)
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_refs_in_pattern(
        &self,
        pattern: &crate::ast::statements::Pattern,
        name: &str,
        refs: &mut Vec<Location>,
    ) {
        match pattern {
            crate::ast::statements::Pattern::Binding(binding, span) => {
                if binding == name {
                    refs.push(self.to_named_location(span, binding));
                }
            }
            crate::ast::statements::Pattern::Literal(lit) => {
                if let Literal::FString(fstring) = lit {
                    for part in &fstring.parts {
                        if let FStringPart::Interpolation(expr) = part {
                            self.collect_refs_in_expr(expr, name, refs);
                        }
                    }
                }
            }
            crate::ast::statements::Pattern::Enum(enum_pattern) => {
                if enum_pattern.name == name {
                    refs.push(self.to_location(&enum_pattern.span));
                }
                for nested in &enum_pattern.patterns {
                    self.collect_refs_in_pattern(nested, name, refs);
                }
            }
            crate::ast::statements::Pattern::Wildcard(_) => {}
        }
    }

    fn collect_refs_in_expr(&self, expr: &Expression, name: &str, refs: &mut Vec<Location>) {
        match expr {
            Expression::Identifier(id, span) => {
                if id == name {
                    refs.push(self.to_location(span));
                }
            }
            Expression::Binary(b) => {
                self.collect_refs_in_expr(&b.left, name, refs);
                self.collect_refs_in_expr(&b.right, name, refs);
            }
            Expression::Unary(u) => {
                self.collect_refs_in_expr(&u.operand, name, refs);
            }
            Expression::Postfix(p) => {
                self.collect_refs_in_expr(&p.base, name, refs);
                for op in &p.ops {
                    match op {
                        PostfixOp::Call(args) => {
                            for arg in args {
                                self.collect_refs_in_expr(arg, name, refs);
                            }
                        }
                        PostfixOp::Index(idx) => {
                            self.collect_refs_in_expr(idx, name, refs);
                        }
                        PostfixOp::ChannelSend(val) => {
                            self.collect_refs_in_expr(val, name, refs);
                        }
                        PostfixOp::Catch(block) => {
                            self.collect_refs_in_block(block, name, refs);
                        }
                        _ => {}
                    }
                }
            }
            Expression::Grouped(g, _) => self.collect_refs_in_expr(g, name, refs),
            Expression::Ternary(t) => {
                self.collect_refs_in_expr(&t.condition, name, refs);
                self.collect_refs_in_expr(&t.then_expr, name, refs);
                self.collect_refs_in_expr(&t.else_expr, name, refs);
            }
            Expression::Literal(Literal::FString(fstring)) => {
                for part in &fstring.parts {
                    if let FStringPart::Interpolation(expr) = part {
                        self.collect_refs_in_expr(expr, name, refs);
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_refs_in_type(&self, ty: &TypeExpr, name: &str, refs: &mut Vec<Location>) {
        if let TypeExpr::Base(b) = ty {
            if b.name == name {
                refs.push(self.to_location(&b.span));
            }
        }
    }
}

fn is_ident_byte(b: u8) -> bool {
    (b as char).is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::analysis::parse_and_collect_diagnostics;
    use tower_lsp::lsp_types::Url;

    fn position_of(content: &str, needle: &str) -> Position {
        let idx = content
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let mut line = 0u32;
        let mut col = 0u32;
        for b in content.as_bytes().iter().take(idx) {
            if *b == b'\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        Position {
            line,
            character: col,
        }
    }

    fn text_for_location(content: &str, location: &Location) -> String {
        let start = position_to_offset(content, location.range.start);
        let end = position_to_offset(content, location.range.end);
        content[start..end].to_string()
    }

    #[test]
    fn test_collect_visible_symbols() {
        let content = r#"
            struct MyStruct {
                a Int
            }

            fn main() {
                x Int = 10
                y Int = 20
                // CURSOR HERE
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse program: {:?}", diags);
        }
        let program = program.unwrap();

        let finder = ReferenceFinder::new(content, &program);

        let position = Position {
            line: 8,
            character: 16,
        };
        let symbols = finder.collect_visible_symbols(position);

        let labels: Vec<String> = symbols.iter().map(|i| i.label.clone()).collect();

        assert!(labels.contains(&"MyStruct".to_string()));
        assert!(labels.contains(&"main".to_string()));
        assert!(labels.contains(&"x".to_string()));
        assert!(labels.contains(&"y".to_string()));
    }

    #[test]
    fn test_symbol_visibility_scope() {
        let content = r#"
            fn main() {
                a Int = 1
                if true {
                    b Int = 2
                    // CURSOR 1
                }
                // CURSOR 2
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse program: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let pos1 = Position {
            line: 5,
            character: 20,
        };
        let symbols1 = finder.collect_visible_symbols(pos1);
        let labels1: Vec<String> = symbols1.iter().map(|i| i.label.clone()).collect();
        assert!(labels1.contains(&"a".to_string()));
        assert!(labels1.contains(&"b".to_string()));

        let pos2 = Position {
            line: 7,
            character: 16,
        };
        let symbols2 = finder.collect_visible_symbols(pos2);
        let labels2: Vec<String> = symbols2.iter().map(|i| i.label.clone()).collect();
        assert!(labels2.contains(&"a".to_string()));
        assert!(!labels2.contains(&"b".to_string()));
    }

    #[test]
    fn test_collect_defines() {
        let content = r#"
            define MY_CONST = 100

            fn main() {
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, _) = parse_and_collect_diagnostics(content, &uri);
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let symbols = finder.collect_defines();
        let labels: Vec<String> = symbols.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"MY_CONST".to_string()));
    }

    #[test]
    fn test_field_completion() {
        let content = r#"
            struct Point {
                pub x Int
                y Int
            }

            fn main() {
                p Point = Point { x: 10, y: 20 }
                p.dummy
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let position = Position {
            line: 8,
            character: 18,
        };
        let items = finder.collect_dot_completions(position);

        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"x".to_string()));
        assert!(!labels.contains(&"y".to_string()));
    }

    #[test]
    fn test_find_references_includes_else_branch_usages() {
        let content = r#"
            fn main() {
                value Int = 1
                if false {
                } else {
                    value = value + 1
                }
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let refs = finder.find_references(position_of(content, "value Int"));
        assert!(
            refs.len() >= 2,
            "references should include else-branch usages"
        );
    }

    #[test]
    fn test_find_references_includes_match_arm_usages() {
        let content = r#"
            fn main() {
                v Int = 1
                match v {
                    1 => {
                        v = v + 1
                    }
                    _ => {}
                }
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let refs = finder.find_references(position_of(content, "v Int"));
        assert!(
            refs.len() >= 2,
            "references should include match-arm usages"
        );
    }

    #[test]
    fn test_find_references_includes_range_index_declaration() {
        let content = r#"
            fn main() {
                mut sum Int = 0
                for i := range [1, 2, 3] {
                    sum = sum + i
                }
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let refs = finder.find_references(position_of(content, "i := range"));
        assert!(
            refs.len() >= 2,
            "references should include range index declaration and use"
        );
    }

    #[test]
    fn test_find_references_by_name_collects_top_level_and_usages() {
        let content = r#"
            define LIMIT Int = 1

            fn a() {
                print(LIMIT)
            }

            fn b() {
                print(LIMIT)
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let refs = finder.find_references_by_name("LIMIT");
        assert!(refs.len() >= 3);
    }

    #[test]
    fn test_reference_ranges_are_identifier_only() {
        let content = r#"
            fn main() {
                junk [100]Int = [0; 100]
                junk[i] = 999
            }
        "#;

        let uri = Url::parse("file:///test.breom").unwrap();
        let (program, diags) = parse_and_collect_diagnostics(content, &uri);
        if program.is_none() {
            panic!("Failed to parse: {:?}", diags);
        }
        let program = program.unwrap();
        let finder = ReferenceFinder::new(content, &program);

        let refs = finder.find_references(position_of(content, "junk [100]Int"));
        assert!(refs.len() >= 2);
        for loc in refs {
            assert_eq!(text_for_location(content, &loc), "junk");
        }
    }
}

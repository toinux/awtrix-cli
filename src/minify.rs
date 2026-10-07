//! Local Berry minification, ported from the project's existing TypeScript utility.
//! The supported class-member rewrite intentionally remains limited to literal `self.field` access.
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn run(
    file: &Path,
    classes: bool,
    variables: bool,
    force: bool,
) -> crate::CliResult<Value> {
    let source = fs::read_to_string(file)
        .map_err(|error| ("MINIFY_INPUT", format!("cannot read UTF-8 source: {error}")))?;
    let output = output_path(file)?;
    if output.exists() && !force {
        return Err((
            "OUTPUT_EXISTS",
            format!(
                "{} already exists; pass --force to replace it",
                output.display()
            ),
        ));
    }
    let compacted = minify_berry(
        &source,
        MinifyOptions {
            rename_locals: variables,
            rename_classes: classes,
            rename_members: variables,
            join_lines: false,
            merge_var_decls: true,
        },
    )?;
    let bytes = compacted.len();
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        (
            "MINIFY_OUTPUT",
            format!(
                "cannot create a temporary output beside {}: {error}",
                output.display()
            ),
        )
    })?;
    use std::io::Write;
    temporary
        .write_all(compacted.as_bytes())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| {
            (
                "MINIFY_OUTPUT",
                format!("cannot write {}: {error}", output.display()),
            )
        })?;
    if force {
        temporary.persist(&output).map_err(|error| {
            (
                "MINIFY_OUTPUT",
                format!("cannot replace {}: {}", output.display(), error.error),
            )
        })?;
    } else {
        temporary.persist_noclobber(&output).map_err(|error| {
            if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                (
                    "OUTPUT_EXISTS",
                    format!(
                        "{} already exists; pass --force to replace it",
                        output.display()
                    ),
                )
            } else {
                (
                    "MINIFY_OUTPUT",
                    format!("cannot create {}: {}", output.display(), error.error),
                )
            }
        })?;
    }
    Ok(json!({"output":output,"bytes":bytes}))
}

fn output_path(file: &Path) -> crate::CliResult<PathBuf> {
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or(("MINIFY_INPUT", "input must have a UTF-8 file name".into()))?;
    let parent = file.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!("{stem}.min.ax")))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind {
    Header,
    Comment,
    String,
    Number,
    Ident,
    Keyword,
    Punct,
    Space,
    Newline,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    value: String,
}

impl Token {
    fn new(kind: TokenKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_part(byte: u8) -> bool {
    is_ident_start(byte) || byte.is_ascii_digit()
}

fn is_trivia(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Comment | TokenKind::Space | TokenKind::Newline
    )
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "if" | "elif"
            | "else"
            | "while"
            | "for"
            | "def"
            | "end"
            | "class"
            | "break"
            | "continue"
            | "return"
            | "true"
            | "false"
            | "nil"
            | "var"
            | "do"
            | "import"
            | "as"
            | "try"
            | "except"
            | "raise"
            | "static"
    )
}

fn tokenize(source: &str) -> crate::CliResult<Vec<Token>> {
    const MULTI_PUNCT: [&str; 21] = [
        "<<=", ">>=", "::=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "==", "!=", "<=",
        ">=", "&&", "||", "..", "->", "<<", ">>",
    ];
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'\n' {
            tokens.push(Token::new(TokenKind::Newline, "\n"));
            i += 1;
            continue;
        }
        if byte == b'\r' {
            if bytes.get(i + 1) == Some(&b'\n') {
                i += 2;
            } else {
                i += 1;
            }
            tokens.push(Token::new(TokenKind::Newline, "\n"));
            continue;
        }
        if matches!(byte, b' ' | b'\t' | b'\x0c' | b'\x0b') {
            let start = i;
            i += 1;
            while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\x0c' | b'\x0b') {
                i += 1;
            }
            tokens.push(Token::new(TokenKind::Space, &source[start..i]));
            continue;
        }
        if byte == b'#' {
            if bytes.get(i + 1) == Some(&b'-') {
                let start = i;
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'-' && bytes[i + 1] == b'#') {
                    i += 1;
                }
                if i + 1 >= bytes.len() {
                    return Err(("MINIFY_SYNTAX", "unterminated block comment".into()));
                }
                i += 2;
                tokens.push(Token::new(TokenKind::Comment, &source[start..i]));
            } else {
                let start = i;
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    i += 1;
                }
                let raw = &source[start..i];
                let trimmed = raw.trim();
                let kind = if trimmed.starts_with("# @") {
                    TokenKind::Header
                } else {
                    TokenKind::Comment
                };
                tokens.push(Token::new(
                    kind,
                    if kind == TokenKind::Header {
                        trimmed
                    } else {
                        raw
                    },
                ));
            }
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            let quote = byte;
            let start = i;
            i += 1;
            let mut closed = false;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == quote {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if !closed {
                return Err(("MINIFY_SYNTAX", "unterminated string literal".into()));
            }
            tokens.push(Token::new(TokenKind::String, &source[start..i]));
            continue;
        }
        if byte == b'0' && matches!(bytes.get(i + 1), Some(b'x' | b'X')) {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            tokens.push(Token::new(TokenKind::Number, &source[start..i]));
            continue;
        }
        if byte.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'.')
                && bytes.get(i + 1) != Some(&b'.')
                && bytes.get(i + 1).is_some_and(|next| next.is_ascii_digit())
            {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if matches!(bytes.get(i), Some(b'e' | b'E')) {
                i += 1;
                if matches!(bytes.get(i), Some(b'+' | b'-')) {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            tokens.push(Token::new(TokenKind::Number, &source[start..i]));
            continue;
        }
        if is_ident_start(byte) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_part(bytes[i]) {
                i += 1;
            }
            let value = &source[start..i];
            let kind = if is_keyword(value) {
                TokenKind::Keyword
            } else {
                TokenKind::Ident
            };
            tokens.push(Token::new(kind, value));
            continue;
        }
        if let Some(operator) = MULTI_PUNCT
            .iter()
            .find(|operator| source[i..].starts_with(**operator))
        {
            tokens.push(Token::new(TokenKind::Punct, *operator));
            i += operator.len();
            continue;
        }
        let ch = source[i..]
            .chars()
            .next()
            .ok_or(("MINIFY_SYNTAX", "invalid UTF-8 boundary".into()))?;
        tokens.push(Token::new(TokenKind::Punct, ch.to_string()));
        i += ch.len_utf8();
    }
    Ok(tokens)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScopeKind {
    Module,
    Class,
    Function,
    Block,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingKind {
    Local,
    Class,
    Function,
    Alias,
    Member,
}

#[derive(Clone, Debug)]
struct Binding {
    rename: bool,
    kind: BindingKind,
    short: Option<String>,
}

#[derive(Debug)]
struct Scope {
    kind: ScopeKind,
    parent: Option<usize>,
    children: Vec<usize>,
    order: Vec<String>,
    bindings: HashMap<String, Binding>,
}

impl Scope {
    fn new(kind: ScopeKind, parent: Option<usize>) -> Self {
        Self {
            kind,
            parent,
            children: Vec::new(),
            order: Vec::new(),
            bindings: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct MinifyOptions {
    rename_locals: bool,
    rename_classes: bool,
    rename_members: bool,
    join_lines: bool,
    merge_var_decls: bool,
}

fn minify_berry(source: &str, options: MinifyOptions) -> crate::CliResult<String> {
    let mut tokens = tokenize(source)?;
    if options.merge_var_decls {
        tokens = merge_class_var_decls(&tokens);
    }
    let (mut scopes, ident_scope) = analyze(&tokens);
    let frozen = collect_frozen(&scopes);
    assign_short_names(&mut scopes, 0, &HashSet::new(), &frozen);
    emit(&tokens, &scopes, &ident_scope, options)
}

fn next_code(tokens: &[Token], mut index: usize) -> usize {
    while index < tokens.len() && is_trivia(&tokens[index]) {
        index += 1;
    }
    index
}

fn previous_code(tokens: &[Token], mut index: usize) -> Option<usize> {
    while index > 0 {
        index -= 1;
        if !is_trivia(&tokens[index]) {
            return Some(index);
        }
    }
    None
}

fn add_scope(scopes: &mut Vec<Scope>, parent: usize, kind: ScopeKind) -> usize {
    let id = scopes.len();
    scopes.push(Scope::new(kind, Some(parent)));
    scopes[parent].children.push(id);
    id
}

fn enclosing_function(scopes: &[Scope], mut id: usize) -> usize {
    loop {
        if scopes[id].kind == ScopeKind::Function {
            return id;
        }
        match scopes[id].parent {
            Some(parent) => id = parent,
            None => return id,
        }
    }
}

fn enclosing_class(scopes: &[Scope], mut id: usize) -> Option<usize> {
    loop {
        if scopes[id].kind == ScopeKind::Class {
            return Some(id);
        }
        id = scopes[id].parent?;
    }
}

fn bind(scopes: &mut [Scope], scope: usize, name: &str, rename: bool, kind: BindingKind) {
    if name.is_empty() || scopes[scope].bindings.contains_key(name) {
        return;
    }
    let frozen = frozen_globals().contains(&name) || frozen_methods().contains(&name);
    scopes[scope].order.push(name.to_owned());
    scopes[scope].bindings.insert(
        name.to_owned(),
        Binding {
            rename: rename && !frozen,
            kind,
            short: None,
        },
    );
}

fn lookup(scopes: &[Scope], scope: Option<usize>, name: &str) -> Option<(usize, Binding)> {
    let mut current = scope;
    while let Some(id) = current {
        if let Some(binding) = scopes[id].bindings.get(name) {
            return Some((id, binding.clone()));
        }
        current = scopes[id].parent;
    }
    None
}

fn collect_names(tokens: &[Token], start: usize) -> (Vec<(usize, String)>, usize) {
    let mut names = Vec::new();
    let mut cursor = next_code(tokens, start);
    loop {
        if cursor >= tokens.len() || tokens[cursor].kind != TokenKind::Ident {
            break;
        }
        names.push((cursor, tokens[cursor].value.clone()));
        cursor = next_code(tokens, cursor + 1);
        if cursor < tokens.len()
            && tokens[cursor].kind == TokenKind::Punct
            && tokens[cursor].value == ","
        {
            cursor = next_code(tokens, cursor + 1);
            continue;
        }
        break;
    }
    (names, cursor)
}

fn is_declaration_equals(tokens: &[Token], ident: usize) -> bool {
    let mut cursor = ident;
    loop {
        if cursor >= tokens.len() || tokens[cursor].kind != TokenKind::Ident {
            return false;
        }
        cursor = next_code(tokens, cursor + 1);
        if cursor >= tokens.len() {
            return false;
        }
        if tokens[cursor].kind == TokenKind::Punct && tokens[cursor].value == "=" {
            return true;
        }
        if tokens[cursor].kind == TokenKind::Punct && tokens[cursor].value == "," {
            cursor = next_code(tokens, cursor + 1);
            continue;
        }
        return false;
    }
}

fn module_file(tokens: &[Token]) -> bool {
    for token in tokens {
        let directive = token.value.trim_start_matches('#').trim_start();
        let module_marker = directive
            .get(..7)
            .is_some_and(|marker| marker.eq_ignore_ascii_case("@module"))
            && directive
                .as_bytes()
                .get(7)
                .is_none_or(|next| !is_ident_part(*next));
        if token.kind == TokenKind::Header && module_marker {
            return true;
        }
        if !matches!(
            token.kind,
            TokenKind::Header | TokenKind::Comment | TokenKind::Space | TokenKind::Newline
        ) {
            return false;
        }
    }
    false
}

fn analyze(tokens: &[Token]) -> (Vec<Scope>, Vec<Option<usize>>) {
    let is_module = module_file(tokens);
    let mut scopes = vec![Scope::new(ScopeKind::Module, None)];
    let mut ident_scope = vec![None; tokens.len()];
    let mut blocks = vec![("module".to_owned(), 0usize)];
    let mut current = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Ident {
            ident_scope[index] = Some(current);
            continue;
        }
        if token.kind != TokenKind::Keyword {
            continue;
        }
        match token.value.as_str() {
            "class" => {
                let name_index = next_code(tokens, index + 1);
                if name_index < tokens.len() && tokens[name_index].kind == TokenKind::Ident {
                    bind(
                        &mut scopes,
                        current,
                        &tokens[name_index].value,
                        !is_module,
                        BindingKind::Class,
                    );
                }
                let child = add_scope(&mut scopes, current, ScopeKind::Class);
                blocks.push(("class".to_owned(), child));
                current = child;
            }
            "def" => {
                let method_level = scopes[enclosing_function(&scopes, current)].kind
                    != ScopeKind::Function
                    && enclosing_class(&scopes, current).is_some();
                let mut cursor = next_code(tokens, index + 1);
                let mut name = None;
                if cursor < tokens.len() && tokens[cursor].kind == TokenKind::Ident {
                    name = Some(tokens[cursor].value.clone());
                    cursor = next_code(tokens, cursor + 1);
                }
                if let Some(name) = name {
                    if method_level || scopes[current].kind == ScopeKind::Class {
                        let class_scope = enclosing_class(&scopes, current).unwrap_or(current);
                        bind(
                            &mut scopes,
                            class_scope,
                            &name,
                            false,
                            BindingKind::Function,
                        );
                    } else if scopes[current].kind == ScopeKind::Module {
                        bind(
                            &mut scopes,
                            current,
                            &name,
                            !is_module,
                            BindingKind::Function,
                        );
                    } else {
                        let function_scope = enclosing_function(&scopes, current);
                        bind(
                            &mut scopes,
                            function_scope,
                            &name,
                            true,
                            BindingKind::Function,
                        );
                    }
                }
                let function = add_scope(&mut scopes, current, ScopeKind::Function);
                if cursor < tokens.len()
                    && tokens[cursor].kind == TokenKind::Punct
                    && tokens[cursor].value == "("
                {
                    let (params, _) = collect_names(tokens, cursor + 1);
                    for (_, name) in params {
                        bind(&mut scopes, function, &name, true, BindingKind::Local);
                    }
                }
                blocks.push(("def".to_owned(), function));
                current = function;
            }
            "var" => {
                let (names, _) = collect_names(tokens, index + 1);
                let in_function =
                    scopes[enclosing_function(&scopes, current)].kind == ScopeKind::Function;
                let class = enclosing_class(&scopes, current);
                let (target, kind) = if let Some(class) = class.filter(|_| !in_function) {
                    (class, BindingKind::Member)
                } else {
                    (enclosing_function(&scopes, current), BindingKind::Local)
                };
                for (_, name) in names {
                    bind(&mut scopes, target, &name, true, kind);
                }
            }
            "static" => {
                let next = next_code(tokens, index + 1);
                if next < tokens.len()
                    && tokens[next].kind == TokenKind::Keyword
                    && tokens[next].value == "def"
                {
                    continue;
                }
                let names_start = if next < tokens.len()
                    && tokens[next].kind == TokenKind::Keyword
                    && tokens[next].value == "var"
                {
                    next + 1
                } else {
                    index + 1
                };
                let (names, _) = collect_names(tokens, names_start);
                let class = enclosing_class(&scopes, current).unwrap_or(current);
                for (_, name) in names {
                    bind(&mut scopes, class, &name, false, BindingKind::Local);
                }
            }
            "for" => {
                let variable = next_code(tokens, index + 1);
                if variable < tokens.len() && tokens[variable].kind == TokenKind::Ident {
                    let function = enclosing_function(&scopes, current);
                    bind(
                        &mut scopes,
                        function,
                        &tokens[variable].value,
                        true,
                        BindingKind::Local,
                    );
                }
                let block = add_scope(&mut scopes, current, ScopeKind::Block);
                blocks.push(("for".to_owned(), block));
                current = block;
            }
            "if" | "while" | "try" => {
                let block = add_scope(&mut scopes, current, ScopeKind::Block);
                blocks.push((token.value.clone(), block));
                current = block;
            }
            "do" => {
                let previous = previous_code(tokens, index);
                let attached = previous.is_some_and(|previous| {
                    tokens[previous].kind == TokenKind::Keyword
                        && matches!(
                            tokens[previous].value.as_str(),
                            "if" | "while" | "for" | "elif" | "else" | "except"
                        )
                });
                if !attached {
                    let block = add_scope(&mut scopes, current, ScopeKind::Block);
                    blocks.push(("do".to_owned(), block));
                    current = block;
                }
            }
            "import" => {
                let module = next_code(tokens, index + 1);
                if module < tokens.len() && tokens[module].kind == TokenKind::Ident {
                    let alias = next_code(tokens, module + 1);
                    if alias < tokens.len()
                        && tokens[alias].kind == TokenKind::Keyword
                        && tokens[alias].value == "as"
                    {
                        let name = next_code(tokens, alias + 1);
                        if name < tokens.len() && tokens[name].kind == TokenKind::Ident {
                            bind(
                                &mut scopes,
                                current,
                                &tokens[name].value,
                                true,
                                BindingKind::Alias,
                            );
                        }
                    } else {
                        bind(
                            &mut scopes,
                            current,
                            &tokens[module].value,
                            false,
                            BindingKind::Alias,
                        );
                    }
                }
            }
            "as" => {
                let name = next_code(tokens, index + 1);
                if name < tokens.len() && tokens[name].kind == TokenKind::Ident {
                    let found = lookup(&scopes, Some(current), &tokens[name].value);
                    if found.is_none() {
                        let function = enclosing_function(&scopes, current);
                        bind(
                            &mut scopes,
                            function,
                            &tokens[name].value,
                            true,
                            BindingKind::Local,
                        );
                    }
                }
            }
            "end" if blocks.len() > 1 => {
                blocks.pop();
                current = blocks.last().map(|(_, id)| *id).unwrap_or(0);
            }
            "end" => {}
            _ => {}
        }
    }

    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Punct || token.value != "/" {
            continue;
        }
        let mut cursor = next_code(tokens, index + 1);
        let mut params = Vec::new();
        while cursor < tokens.len() && tokens[cursor].kind == TokenKind::Ident {
            params.push(tokens[cursor].value.clone());
            cursor = next_code(tokens, cursor + 1);
            if cursor < tokens.len()
                && tokens[cursor].kind == TokenKind::Punct
                && tokens[cursor].value == ","
            {
                cursor = next_code(tokens, cursor + 1);
            }
        }
        if !params.is_empty()
            && cursor < tokens.len()
            && tokens[cursor].kind == TokenKind::Punct
            && tokens[cursor].value == "->"
        {
            if let Some(scope) = ident_scope[index] {
                let function = enclosing_function(&scopes, scope);
                for param in params {
                    bind(&mut scopes, function, &param, true, BindingKind::Local);
                }
            }
        }
    }

    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Ident
            || previous_code(tokens, index).is_some_and(|previous| {
                tokens[previous].kind == TokenKind::Punct && tokens[previous].value == "."
            })
            || !is_declaration_equals(tokens, index)
        {
            continue;
        }
        let Some(scope) = ident_scope[index] else {
            continue;
        };
        let function = enclosing_function(&scopes, scope);
        if scopes[function].kind != ScopeKind::Function
            || lookup(&scopes, Some(scope), &token.value).is_some()
            || frozen_globals().contains(&token.value.as_str())
            || frozen_methods().contains(&token.value.as_str())
        {
            continue;
        }
        bind(
            &mut scopes,
            function,
            &token.value,
            true,
            BindingKind::Local,
        );
    }
    (scopes, ident_scope)
}

fn merge_class_var_decls(tokens: &[Token]) -> Vec<Token> {
    let mut output = Vec::new();
    let mut stack = vec![ScopeKind::Module];
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index].kind == TokenKind::Keyword
            && tokens[index].value == "var"
            && stack.last() == Some(&ScopeKind::Class)
        {
            let mut cursor = index;
            let mut names = Vec::new();
            let mut valid = true;
            loop {
                if cursor >= tokens.len()
                    || tokens[cursor].kind != TokenKind::Keyword
                    || tokens[cursor].value != "var"
                {
                    valid = false;
                    break;
                }
                let (declaration, end) = collect_names(tokens, cursor + 1);
                if declaration.is_empty()
                    || (end < tokens.len()
                        && tokens[end].kind == TokenKind::Punct
                        && tokens[end].value == "=")
                {
                    valid = false;
                    break;
                }
                names.extend(declaration.into_iter().map(|(_, name)| name));
                cursor = next_code(tokens, end);
                if cursor < tokens.len()
                    && tokens[cursor].kind == TokenKind::Keyword
                    && tokens[cursor].value == "var"
                {
                    continue;
                }
                break;
            }
            if valid && !names.is_empty() {
                output.push(Token::new(TokenKind::Keyword, "var"));
                output.push(Token::new(TokenKind::Space, " "));
                for (name_index, name) in names.into_iter().enumerate() {
                    if name_index > 0 {
                        output.push(Token::new(TokenKind::Punct, ","));
                    }
                    output.push(Token::new(TokenKind::Ident, name));
                }
                output.push(Token::new(TokenKind::Newline, "\n"));
                index = cursor;
                continue;
            }
        }
        let token = &tokens[index];
        if token.kind == TokenKind::Keyword {
            match token.value.as_str() {
                "class" => stack.push(ScopeKind::Class),
                "def" => stack.push(ScopeKind::Function),
                "if" | "for" | "while" | "try" => stack.push(ScopeKind::Block),
                "do" => {
                    let previous = previous_code(tokens, index);
                    let attached = previous.is_some_and(|previous| {
                        tokens[previous].kind == TokenKind::Keyword
                            && matches!(
                                tokens[previous].value.as_str(),
                                "if" | "while" | "for" | "elif" | "else" | "except"
                            )
                    });
                    if !attached {
                        stack.push(ScopeKind::Block);
                    }
                }
                "end" if stack.len() > 1 => {
                    stack.pop();
                }
                "end" => {}
                _ => {}
            }
        }
        output.push(token.clone());
        index += 1;
    }
    output
}

fn frozen_globals() -> &'static [&'static str] {
    &[
        "self",
        "super",
        "nil",
        "true",
        "false",
        "print",
        "format",
        "int",
        "real",
        "bool",
        "str",
        "bytes",
        "size",
        "type",
        "classname",
        "classof",
        "isinstance",
        "number",
        "map",
        "list",
        "range",
        "module",
        "globals",
        "compiled",
        "gc",
        "input",
        "assert",
        "open",
        "json",
        "math",
        "string",
        "time",
        "os",
        "debug",
        "introspect",
        "solidify",
        "undefined",
        "comptr",
        "comobj",
        "function",
        "instance",
        "class",
        "store",
        "http",
        "mqtt",
        "re",
        "rotation",
        "sensor",
        "settings",
        "shared",
        "sound",
        "log",
        "notify",
        "num",
        "round",
        "clamp",
        "min",
        "max",
        "width",
        "height",
        "clear",
        "pixel",
        "line",
        "rect",
        "rect_fill",
        "circle",
        "circle_fill",
        "rgb",
        "hsv",
        "text",
        "text_width",
        "text_ink_width",
        "font",
        "ramp_text",
        "scroll_text",
        "bar_chart",
        "line_chart",
        "progress",
        "effect",
        "overlay",
        "icon",
        "hour",
        "minute",
        "second",
        "weekday",
        "day",
        "month",
        "year",
        "epoch_ms",
        "now_ms",
        "version",
    ]
}

fn frozen_methods() -> &'static [&'static str] {
    &[
        "init",
        "deinit",
        "tostring",
        "setup",
        "draw",
        "loop",
        "on_show",
        "on_hide",
        "on_button",
        "should_show",
        "duration",
        "item",
        "setitem",
        "member",
        "setmember",
        "call",
        "add",
        "sub",
        "mul",
        "div",
        "mod",
        "and",
        "or",
        "xor",
        "lsh",
        "rsh",
        "neg",
        "flip",
        "lt",
        "le",
        "eq",
        "ne",
        "gt",
        "ge",
    ]
}

fn collect_frozen(scopes: &[Scope]) -> HashSet<String> {
    let mut frozen = frozen_globals()
        .iter()
        .chain(frozen_methods().iter())
        .copied()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    for scope in scopes {
        for (name, binding) in &scope.bindings {
            if !binding.rename {
                frozen.insert(name.clone());
            }
        }
    }
    frozen
}

struct ShortNameGenerator {
    next: usize,
}

impl ShortNameGenerator {
    fn new() -> Self {
        Self { next: 0 }
    }

    fn next(&mut self) -> String {
        let letters = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let index = self.next;
        self.next += 1;
        let mut name = String::from(char::from(letters[index % letters.len()]));
        if index >= letters.len() {
            name.push_str(&(index / letters.len()).to_string());
        }
        name
    }
}

fn assign_short_names(
    scopes: &mut [Scope],
    scope_id: usize,
    inherited: &HashSet<String>,
    frozen: &HashSet<String>,
) {
    let mut taken = inherited.clone();
    let mut generator = ShortNameGenerator::new();
    let order = scopes[scope_id].order.clone();
    for name in order {
        let Some(binding) = scopes[scope_id].bindings.get(&name).cloned() else {
            continue;
        };
        if !binding.rename {
            taken.insert(name);
            continue;
        }
        let mut short = generator.next();
        while taken.contains(&short) || frozen.contains(&short) {
            short = generator.next();
        }
        taken.insert(short.clone());
        if let Some(binding) = scopes[scope_id].bindings.get_mut(&name) {
            binding.short = Some(short);
        }
    }
    let children = scopes[scope_id].children.clone();
    for child in children {
        assign_short_names(scopes, child, &taken, frozen);
    }
}

fn wordish(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Ident | TokenKind::Keyword | TokenKind::Number
    )
}

fn needs_space(previous: &Token, next: &Token) -> bool {
    (wordish(previous) && wordish(next))
        || (previous.kind == TokenKind::Number && next.kind == TokenKind::Ident)
        || (previous.kind == TokenKind::Ident && next.kind == TokenKind::Number)
}

fn is_after_dot(tokens: &[Token], index: usize) -> Option<usize> {
    let dot = previous_code(tokens, index)?;
    if tokens[dot].kind != TokenKind::Punct || tokens[dot].value != "." {
        return None;
    }
    let object = previous_code(tokens, dot)?;
    Some(object)
}

fn mapped_identifier(
    tokens: &[Token],
    scopes: &[Scope],
    ident_scope: &[Option<usize>],
    index: usize,
    options: MinifyOptions,
) -> String {
    let token = &tokens[index];
    let scope = ident_scope[index];
    let binding = if let Some(object) = is_after_dot(tokens, index) {
        if tokens[object].kind != TokenKind::Ident || tokens[object].value != "self" {
            return token.value.clone();
        }
        let Some(class) = scope.and_then(|scope| enclosing_class(scopes, scope)) else {
            return token.value.clone();
        };
        scopes[class].bindings.get(&token.value).cloned()
    } else {
        lookup(scopes, scope, &token.value).map(|(_, binding)| binding)
    };
    let Some(binding) = binding else {
        return token.value.clone();
    };
    if !binding.rename {
        return token.value.clone();
    }
    let enabled = match binding.kind {
        BindingKind::Local | BindingKind::Alias => options.rename_locals,
        BindingKind::Member => options.rename_members,
        BindingKind::Class | BindingKind::Function => options.rename_classes,
    };
    if !enabled {
        return token.value.clone();
    }
    binding.short.unwrap_or_else(|| token.value.clone())
}

fn emit(
    tokens: &[Token],
    scopes: &[Scope],
    ident_scope: &[Option<usize>],
    options: MinifyOptions,
) -> crate::CliResult<String> {
    let mut output = String::new();
    let mut last: Option<Token> = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Comment | TokenKind::Space => continue,
            TokenKind::Header => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(token.value.trim());
                output.push('\n');
                last = Some(token.clone());
            }
            TokenKind::Newline => {
                if !options.join_lines
                    && last.as_ref().is_some_and(|last| {
                        last.kind != TokenKind::Header && last.kind != TokenKind::Newline
                    })
                {
                    output.push('\n');
                    last = Some(token.clone());
                }
            }
            TokenKind::Ident => {
                let value = mapped_identifier(tokens, scopes, ident_scope, index, options);
                let rendered = Token::new(TokenKind::Ident, value);
                if last
                    .as_ref()
                    .is_some_and(|previous| needs_space(previous, &rendered))
                {
                    output.push(' ');
                }
                output.push_str(&rendered.value);
                last = Some(rendered);
            }
            _ => {
                if last
                    .as_ref()
                    .is_some_and(|previous| needs_space(previous, token))
                {
                    output.push(' ');
                }
                output.push_str(&token.value);
                last = Some(token.clone());
            }
        }
    }
    let mut compacted = String::with_capacity(output.len());
    let mut previous_newline = false;
    for line in output.lines() {
        let line = line.trim_end_matches([' ', '\t']);
        if line.is_empty() && previous_newline {
            continue;
        }
        compacted.push_str(line);
        compacted.push('\n');
        previous_newline = line.is_empty();
    }
    if compacted.is_empty() {
        compacted.push('\n');
    }
    Ok(compacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> MinifyOptions {
        MinifyOptions {
            rename_locals: true,
            rename_classes: true,
            rename_members: true,
            join_lines: false,
            merge_var_decls: true,
        }
    }

    #[test]
    fn preserves_known_and_unknown_metadata_headers() {
        let output = minify_berry(
            "# @name demo\n# @mystery keep\nclass Demo\nend\n",
            default_options(),
        )
        .unwrap();
        assert!(output.starts_with("# @name demo\n# @mystery keep\n"));
        assert!(!output.contains("class Demo"));
    }

    #[test]
    fn renames_top_level_classes_locals_and_literal_self_fields_consistently() {
        let output = minify_berry(
            "class Demo\n  var value\n  def draw()\n    local = 2\n    self.value = local\n    print(self.value)\n  end\nend\nreturn Demo()\n",
            default_options(),
        )
        .unwrap();
        assert!(!output.contains("class Demo"));
        assert!(!output.contains("return Demo"));
        assert!(!output.contains("self.value"));
        assert!(output.contains("def draw"));
        assert!(output.contains("print("));
    }

    #[test]
    fn does_not_rename_non_self_member_accesses() {
        let output = minify_berry(
            "class Demo\n  var value\n  def draw(other)\n    self.value = 1\n    other.value = 2\n  end\nend\n",
            default_options(),
        )
        .unwrap();
        assert!(output.contains(".value"));
        assert!(!output.contains("self.value"));
    }

    #[test]
    fn preserves_strings_operators_and_spacing_between_words() {
        let output = minify_berry(
            "def draw(first, second)\n  text(0, 0, \"first second\")\n  var count = first + second\nend\n",
            default_options(),
        )
        .unwrap();
        assert!(output.contains("\"first second\""));
        assert!(output.contains('+'));
        assert!(!output.contains("var count"));
    }

    #[test]
    fn handles_imports_loops_lambdas_conditionals_chains_and_bitwise_operators() {
        let source = "# @display 32x8\nimport re as regex\nclass Demo\n  var value\n  def draw(items)\n    for item in items\n      self.value = item\n    end\n    mapped = /item -> item + 1\n    choice = mapped > 0 ? self.value : 0\n    mask = (1 << 2) | (3 & 1)\n    text(0, 0, regex.match(\"value\", self.value))\n    return choice + mask\n  end\nend\nreturn Demo()\n";
        let output = minify_berry(source, default_options()).unwrap();
        assert!(output.starts_with("# @display 32x8\nimport"));
        assert!(output.contains("for "));
        assert!(output.contains("->"));
        assert!(output.contains("?"));
        assert!(output.contains("<<"));
        assert!(output.contains("|"));
        assert!(output.contains(".match"));
        assert!(!output.contains("regex"));
        assert!(output.contains("\"value\""));
        assert!(!output.contains("class Demo"));
        assert!(!output.contains("self.value"));
    }

    #[test]
    fn compound_syntax_matches_reference_renaming_and_preserves_unknown_directives() {
        let source = "# @display 32x8\nimport re as regex\nclass Demo\n  var value\n  def draw(items)\n    for item in items\n      self.value = item\n    end\n    mapped = /item -> item + 1\n    choice = mapped > 0 ? self.value : 0\n    mask = (1 << 2) | (3 & 1)\n    text(0, 0, regex.match(\"value\", self.value))\n    return choice + mask\n  end\nend\nreturn Demo()\n";
        let expected = "# @display 32x8\nimport re as a\nclass b\nvar c\ndef draw(d)\nfor item in d\nself.c=item\nend\ne=/item->item+1\nf=e>0?self.c:0\ng=(1<<2)|(3&1)\ntext(0,0,a.match(\"value\",self.c))\nreturn f+g\nend\nend\nreturn b()\n";
        assert_eq!(minify_berry(source, default_options()).unwrap(), expected);
    }

    #[test]
    fn renames_top_level_functions_and_their_parameters_consistently() {
        let output = minify_berry(
            "def calculate(value)\n  return value\nend\nreturn calculate(2)\n",
            default_options(),
        )
        .unwrap();
        assert_eq!(output, "def a(b)\nreturn b\nend\nreturn a(2)\n");
    }

    #[test]
    fn merges_consecutive_class_field_declarations() {
        let output = minify_berry(
            "class Demo\n  var first\n  var third\n  def draw()\n    return self.first + self.third\n  end\nend\n",
            default_options(),
        )
        .unwrap();
        assert!(output.contains("var b,c\n"), "{output}");
        assert!(!output.contains("var first"));
        assert!(!output.contains("var third"));
    }

    #[test]
    fn rejects_unterminated_strings_and_block_comments() {
        assert!(minify_berry("var x = 'unfinished", default_options()).is_err());
        assert!(minify_berry("#- unfinished", default_options()).is_err());
    }

    #[test]
    fn does_not_rename_module_exports() {
        let output = minify_berry(
            "# @MODULE helper\nclass Helper\nend\nreturn Helper\n",
            default_options(),
        )
        .unwrap();
        assert!(output.contains("class Helper"));
        assert!(output.contains("return Helper"));
    }

    #[test]
    fn preserves_static_class_fields() {
        let output = minify_berry(
            "class Demo\n  static var sharedValue\n  def draw()\n    return self.sharedValue\n  end\nend\n",
            default_options(),
        )
        .unwrap();
        assert!(output.contains("static var sharedValue"), "{output}");
        assert!(output.contains("self.sharedValue"));
    }
}

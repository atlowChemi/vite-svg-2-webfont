#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TemplateDependencies {
    pub names: bool,
    pub codepoints: bool,
    pub src: bool,
    pub styles: bool,
    pub dynamic: bool,
}

impl TemplateDependencies {
    pub(crate) const fn css_default() -> Self {
        Self {
            names: false,
            codepoints: true,
            src: true,
            styles: false,
            dynamic: false,
        }
    }

    pub(crate) const fn html_default() -> Self {
        Self {
            names: true,
            codepoints: false,
            src: false,
            styles: true,
            dynamic: false,
        }
    }

    pub(crate) const fn may_depend_on_src(self) -> bool {
        self.src || self.dynamic
    }

    pub(crate) const fn can_reuse_css_no_urls(
        self,
        names_unchanged: bool,
        codepoints_unchanged: bool,
    ) -> bool {
        !self.dynamic
            && !self.src
            && (!self.names || names_unchanged)
            && (!self.codepoints || codepoints_unchanged)
    }

    pub(crate) const fn can_reuse_css_with_urls(
        self,
        names_unchanged: bool,
        codepoints_unchanged: bool,
    ) -> bool {
        !self.dynamic
            && (!self.names || names_unchanged)
            && (!self.codepoints || codepoints_unchanged)
    }

    pub(crate) const fn can_reuse_html(
        self,
        names_unchanged: bool,
        codepoints_unchanged: bool,
        styles_unchanged: bool,
    ) -> bool {
        !self.dynamic
            && (!self.names || names_unchanged)
            && (!self.codepoints || codepoints_unchanged)
            && (!self.styles || styles_unchanged)
    }
}

pub(crate) fn template_dependencies(source: &str) -> TemplateDependencies {
    let mut deps = TemplateDependencies::default();
    for expression in mustache_expressions(source) {
        collect_expression_dependencies(expression, &mut deps);
        if deps.dynamic {
            break;
        }
    }
    deps
}

fn mustache_expressions(mut source: &str) -> Vec<&str> {
    let mut expressions = Vec::new();
    loop {
        let Some(open) = source.find("{{") else {
            return expressions;
        };
        source = &source[open + 2..];
        let (inner_start, close_pat) = match source.strip_prefix('{') {
            Some(rest) => (rest, "}}}"),
            None => (source, "}}"),
        };
        let Some(close) = inner_start.find(close_pat) else {
            return expressions;
        };
        expressions.push(inner_start[..close].trim());
        source = &inner_start[close + close_pat.len()..];
    }
}

fn collect_expression_dependencies(expression: &str, deps: &mut TemplateDependencies) {
    let expression = expression.trim().trim_matches('~').trim();
    if expression.is_empty() || expression.starts_with('!') || expression.starts_with('/') {
        return;
    }
    if matches!(expression, "." | "this" | "@root") || expression.starts_with('>') {
        deps.dynamic = true;
        return;
    }
    if expression.contains('[') || expression.contains(']') {
        deps.dynamic = true;
        return;
    }
    if expression.contains('|') {
        deps.dynamic = true;
        return;
    }
    let tokens = expression.split_whitespace().collect::<Vec<_>>();
    let Some(first) = tokens.first().copied() else {
        return;
    };
    let first = first.trim_start_matches(['#', '^']);
    if first.starts_with('>') {
        deps.dynamic = true;
        return;
    }
    if tokens.iter().any(|token| is_lookup_token(token)) {
        deps.dynamic = true;
        return;
    }
    if matches!(first, "each" | "if" | "unless" | "with") {
        for token in tokens.iter().skip(1) {
            collect_path_dependency(token, deps);
        }
        return;
    }
    if tokens.len() == 1 {
        collect_path_dependency(first, deps);
        return;
    }
    if first == "removePeriods" {
        for token in tokens.iter().skip(1) {
            collect_path_dependency(token, deps);
        }
        return;
    }
    deps.dynamic = true;
}

fn is_lookup_token(token: &str) -> bool {
    token
        .trim_matches(|c| matches!(c, '(' | ')' | ',' | '"' | '\'' | '~'))
        .trim_start_matches(['#', '^'])
        == "lookup"
}

fn collect_path_dependency(path: &str, deps: &mut TemplateDependencies) {
    let mut path = path
        .trim_matches(|c| matches!(c, '(' | ')' | ',' | '"' | '\'' | '~'))
        .trim_start_matches('&')
        .trim_start_matches("../")
        .trim_start_matches("./");
    path = path
        .strip_prefix("@root.")
        .or_else(|| path.strip_prefix("@root/"))
        .unwrap_or(path);
    path = path
        .strip_prefix("this.")
        .or_else(|| path.strip_prefix("this/"))
        .unwrap_or(path);
    if matches!(path, "" | "." | "this" | "@root") {
        deps.dynamic = true;
        return;
    }
    if path.starts_with('@') {
        return;
    }
    let root = path.split(['.', '/']).next().unwrap_or(path);
    match root {
        "names" => deps.names = true,
        "codepoints" => deps.codepoints = true,
        "src" => deps.src = true,
        "styles" => deps.styles = true,
        _ => {}
    }
}

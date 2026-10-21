//! Подстановка значений в шаблоны: {{var}} → значение.

/// Замена всех `{{var}}` в шаблоне на значения из списка пар (имя, значение).
pub fn substitute(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_string();

    for (name, value) in vars.iter() {
        // {{name}} собирается конкатенацией, чтобы не путаться со скобками format!
        let pattern = ["{{", name, "}}"].concat();
        result = result.replace(&pattern, value);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_multiple_vars() {
        let out = substitute("a={{x}} b={{y}} a={{x}}", &[("x", "1"), ("y", "2")]);
        assert_eq!(out, "a=1 b=2 a=1");
    }

    #[test]
    fn missing_vars_left_intact() {
        assert_eq!(
            substitute("{{known}} {{unknown}}", &[("known", "v")]),
            "v {{unknown}}"
        );
    }
}

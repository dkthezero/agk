use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Scope {
    #[default]
    Global,
    Workspace,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_clone() {
        assert_eq!(Scope::Global.clone(), Scope::Global);
    }
}

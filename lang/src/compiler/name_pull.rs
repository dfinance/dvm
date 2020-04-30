const PULL: &str =
    "1234567890qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM/|{}[]():;*&^$#@=-+";

/// Pull of unique names.
/// Contains 3321 unique values.
pub struct NamePull {
    index: usize,
    offset: usize,
}

impl NamePull {
    /// Create new name pull.
    pub fn new() -> Self {
        NamePull {
            index: 0,
            offset: 0,
        }
    }

    /// Gets next value.
    pub fn next(&mut self) -> Option<&'static str> {
        if self.index + self.offset + 1 > PULL.len() {
            None
        } else {
            let val = &PULL[self.index..self.index + self.offset + 1];
            self.index = (self.index + 1) % (PULL.len() - self.offset);
            if self.index == 0 {
                self.offset += 1;
            }
            Some(val)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::compiler::name_pull::NamePull;

    #[test]
    pub fn test_single_char() {
        let mut pull = NamePull::new();
        let mut values = HashSet::new();

        while let Some(val) = pull.next() {
            assert!(values.insert(val));
        }

        assert_eq!(values.len(), 3321);
    }
}

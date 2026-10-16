#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    pub bs: usize,
    pub insert: String,
    pub suffix: usize,
}

pub fn diff_sync(old: &str, new: &str) -> DiffResult {
    let old_c: Vec<char> = old.chars().collect();
    let new_c: Vec<char> = new.chars().collect();
    let mut p = 0usize;
    while p < old_c.len().min(new_c.len()) && old_c[p] == new_c[p] {
        p += 1;
    }
    let mut s = 0usize;
    let max_s = (old_c.len() - p).min(new_c.len() - p);
    while s < max_s && old_c[old_c.len() - 1 - s] == new_c[new_c.len() - 1 - s] {
        s += 1;
    }
    let bs = old_c.len() - p - s;
    let insert: String = new_c[p..new_c.len() - s].iter().collect();
    DiffResult { bs, insert, suffix: s }
}

pub fn key_commands(d: &DiffResult) -> Vec<crate::keys::KeyCommand> {
    use crate::keys::KeyCommand;
    let mut cmds = Vec::with_capacity(4);
    if d.suffix > 0 {
        cmds.push(KeyCommand::MoveLeft { n: d.suffix as i64 });
    }
    if d.bs > 0 {
        cmds.push(KeyCommand::Backspace { n: d.bs as i64 });
    }
    if !d.insert.is_empty() {
        cmds.push(KeyCommand::Type { text: d.insert.clone() });
    }
    if d.suffix > 0 {
        cmds.push(KeyCommand::MoveRight { n: d.suffix as i64 });
    }
    cmds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_diff() {
        let d = diff_sync("ABC", "ABC");
        assert_eq!(d.bs, 0);
        assert_eq!(d.insert, "");
        assert_eq!(d.suffix, 0);
    }

    #[test]
    fn test_pure_insert() {
        let d = diff_sync("ABC", "ABXC");
        assert_eq!(d.bs, 0);
        assert_eq!(d.insert, "X");
        assert_eq!(d.suffix, 1);
    }

    #[test]
    fn test_pure_delete() {
        let d = diff_sync("ABXC", "ABC");
        assert_eq!(d.bs, 1);
        assert_eq!(d.insert, "");
        assert_eq!(d.suffix, 1);
    }

    #[test]
    fn test_middle_replace() {
        let d = diff_sync("ABC", "AXC");
        assert_eq!(d.bs, 1);
        assert_eq!(d.insert, "X");
        assert_eq!(d.suffix, 1);
    }

    #[test]
    fn test_overlap_boundary() {
        let d = diff_sync("AA", "A");
        assert_eq!(d.bs, 1);
        assert_eq!(d.insert, "");
        assert_eq!(d.suffix, 0);
    }

    #[test]
    fn test_unicode_codepoints() {
        let d = diff_sync("你好", "你好世界");
        assert_eq!(d.bs, 0);
        assert_eq!(d.insert, "世界");
        assert_eq!(d.suffix, 0);
        let d2 = diff_sync("hi!", "hi!😀");
        assert_eq!(d2.insert, "😀");
    }

    #[test]
    fn test_empty_inputs() {
        let d = diff_sync("", "");
        assert_eq!(d.bs, 0);
        assert_eq!(d.insert, "");
        assert_eq!(d.suffix, 0);
        let d2 = diff_sync("", "abc");
        assert_eq!(d2.insert, "abc");
        let d3 = diff_sync("abc", "");
        assert_eq!(d3.bs, 3);
    }

    #[test]
    fn test_key_commands_sequence() {
        let d = diff_sync("ABCDEF", "ABXYEF");
        let cmds = key_commands(&d);
        assert_eq!(d.bs, 2);
        assert_eq!(d.insert, "XY");
        assert_eq!(d.suffix, 2);
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn test_suffix_only_edit() {
        let d = diff_sync("ABC", "AX");
        assert_eq!(d.suffix, 0, "后缀不得越过前缀（重叠约束）");
        assert_eq!(d.bs, 2);
        assert_eq!(d.insert, "X");
    }
}

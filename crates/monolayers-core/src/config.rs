use std::path::Path;

/// Check whether a path falls inside an exempt folder (e.g. `_working`).
/// Shared by both server and client — the exemption logic is identical.
pub fn is_exempt(path: &Path, exempt_names: &[String]) -> bool {
    for entry in path.ancestors() {
        if let Some(name) = entry.file_name().and_then(|n| n.to_str())
            && exempt_names.iter().any(|e| e == name)
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(s: &[&str]) -> Vec<String> {
        s.iter().map(|&n| n.to_string()).collect()
    }

    #[test]
    fn file_inside_exempt_folder() {
        let path = Path::new("/sync/_working/doc.txt");
        assert!(is_exempt(path, &names(&["_working"])));
    }

    #[test]
    fn file_nested_inside_exempt_folder() {
        let path = Path::new("/sync/_working/subdir/doc.txt");
        assert!(is_exempt(path, &names(&["_working"])));
    }

    #[test]
    fn file_outside_exempt_folder() {
        let path = Path::new("/sync/documents/doc.txt");
        assert!(!is_exempt(path, &names(&["_working"])));
    }

    #[test]
    fn exempt_folder_itself() {
        let path = Path::new("/sync/_working");
        assert!(is_exempt(path, &names(&["_working"])));
    }

    #[test]
    fn multiple_exempt_names() {
        let exempt = names(&["_working", "_drafts"]);
        assert!(is_exempt(Path::new("/sync/_drafts/file.txt"), &exempt));
        assert!(!is_exempt(Path::new("/sync/final/file.txt"), &exempt));
    }

    #[test]
    fn empty_exempt_list() {
        assert!(!is_exempt(Path::new("/sync/_working/file.txt"), &[]));
    }
}

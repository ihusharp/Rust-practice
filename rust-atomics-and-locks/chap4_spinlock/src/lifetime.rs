pub struct StrSplit<'a, D> {
    remainder: Option<&'a str>,
    delimiter: D,
}

impl<'a, D> StrSplit<'a, D> {
    pub fn new(haystack: &'a str, delimiter: D) -> Self {
        Self {
            remainder: Some(haystack),
            delimiter,
        }
    }
}

impl<'a, D> Iterator for StrSplit<'a, D>
where
    D: Delimiter,
{
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let remainder = self.remainder.as_mut()?;
        if let Some((delim_start, delim_end)) = self.delimiter.find_next(remainder) {
            let until_delimiter = &remainder[..delim_start];
            *remainder = &remainder[delim_end..];
            Some(until_delimiter)
        } else {
            self.remainder.take()
        }
    }
}

trait Delimiter {
    fn find_next(&self, s: &str) -> Option<(usize, usize)>;
}

impl Delimiter for &str {
    fn find_next(&self, s: &str) -> Option<(usize, usize)> {
        s.find(self).map(|start| (start, start + self.len()))
    }
}

impl Delimiter for char {
    fn find_next(&self, s: &str) -> Option<(usize, usize)> {
        s.char_indices()
            .find(|(_, c)| c == self)
            .map(|(delim_start, _)| (delim_start, delim_start + self.len_utf8()))
    }
}

#[allow(dead_code)]
fn until_char(s: &str, c: char) -> &str {
    StrSplit::new(s, c)
        .next()
        .expect("StrSplit always gives at least one result")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn until_char_test() {
        assert_eq!(until_char("hello world", 'o'), "hell");
    }

    #[test]
    fn it_works() {
        let haystack = "a b c d e";
        let letters = StrSplit::new(haystack, " ");
        assert!(letters.eq(vec!["a", "b", "c", "d", "e"].into_iter()));
    }

    #[test]
    fn tail() {
        let haystack = "a b c d ";
        let letters = StrSplit::new(haystack, " ");
        // letters.for_each(|x| println!("{}", x));
        assert!(letters.eq(vec!["a", "b", "c", "d", ""].into_iter()));
    }

    #[test]
    fn tail_no_delimiter() {
        let haystack = "a b c d";
        let letters = StrSplit::new(haystack, " ");
        assert!(letters.eq(vec!["a", "b", "c", "d"].into_iter()));
    }
}

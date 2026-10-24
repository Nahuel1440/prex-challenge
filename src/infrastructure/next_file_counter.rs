//! Finds the number the next cut-off file gets, from the files already in the folder.

/// Number given to the first cut-off file when the folder holds none.
const FIRST_FILE_COUNTER: u64 = 1;

/// Digits of the `DDMMYYYY` part of a cut-off file name.
const DATE_DIGITS: usize = 8;
/// Separates the date from the counter in a cut-off file name.
const FILE_COUNTER_SEPARATOR: char = '_';
/// Ending of every cut-off file name.
const FILE_EXTENSION: &str = ".DAT";

/// The number after the highest `N` among names like `DDMMYYYY_N.DAT`, of any date.
///
/// Names that do not follow that pattern, such as temp files, are ignored.
pub fn next_file_counter<'a>(file_names: impl Iterator<Item = &'a str>) -> u64 {
    file_names
        .filter_map(file_counter_of)
        .max()
        .map_or(FIRST_FILE_COUNTER, |highest| highest + 1)
}

/// The `N` of a name like `24102026_7.DAT`, or `None` for any other name.
fn file_counter_of(file_name: &str) -> Option<u64> {
    let (date, counter_and_extension) = file_name.split_once(FILE_COUNTER_SEPARATOR)?;
    let counter = counter_and_extension.strip_suffix(FILE_EXTENSION)?;

    (date.len() == DATE_DIGITS && is_all_digits(date) && is_all_digits(counter))
        .then(|| counter.parse().ok())
        .flatten()
}

/// True for a non-empty text made only of the digits 0 to 9.
fn is_all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_file_counter_is_one_when_there_are_no_files() {
        assert_eq!(next_file_counter(std::iter::empty()), 1);
    }

    #[test]
    fn next_file_counter_continues_after_the_highest_number_of_any_date() {
        let names = ["24102026_1.DAT", "01012027_7.DAT", "30112026_3.DAT"];

        assert_eq!(next_file_counter(names.into_iter()), 8);
    }

    #[test]
    fn next_file_counter_ignores_temp_files_and_other_names() {
        let names = [".tmp-24102026_9.DAT.tmp", "notes.txt", "24102026_2.DAT"];

        assert_eq!(next_file_counter(names.into_iter()), 3);
    }

    #[test]
    fn next_file_counter_ignores_names_with_a_malformed_date_or_extension() {
        let names = [
            "2410206_9.DAT",
            "241020266_9.DAT",
            "24102026_9.dat",
            "24102026_.DAT",
            "24102026_x.DAT",
            "24102026_9.DAT.bak",
        ];

        assert_eq!(next_file_counter(names.into_iter()), 1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retention {
    pub keep: u64,
    pub every: u64,
}

impl Retention {
    #[must_use]
    pub fn excess(self, held: u64) -> u64 {
        let over = held.saturating_sub(self.keep);
        if over < self.every.max(1) { 0 } else { over }
    }
}

#[cfg(test)]
mod tests {
    use super::Retention;

    #[test]
    fn nothing_is_dropped_until_a_whole_batch_is_over() {
        let retention = Retention { keep: 10, every: 5 };
        assert_eq!(retention.excess(10), 0);
        assert_eq!(retention.excess(14), 0);
        assert_eq!(retention.excess(15), 5);
        assert_eq!(retention.excess(40), 30);
    }

    #[test]
    fn a_batch_of_zero_or_one_drops_every_extra_record() {
        assert_eq!(Retention { keep: 10, every: 1 }.excess(11), 1);
        assert_eq!(Retention { keep: 10, every: 0 }.excess(11), 1);
    }
}

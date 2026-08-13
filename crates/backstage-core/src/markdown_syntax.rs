#[derive(Clone, Copy, Debug)]
struct MarkdownFence {
    marker: char,
    length: usize,
}

#[derive(Default)]
pub(crate) struct FenceTracker {
    active: Option<MarkdownFence>,
}

impl FenceTracker {
    pub(crate) fn consume(&mut self, line: &str) -> bool {
        let Some((marker, length, remainder)) = fence_run(line) else {
            return false;
        };
        match self.active {
            Some(active)
                if marker == active.marker
                    && length >= active.length
                    && remainder.trim().is_empty() =>
            {
                self.active = None;
                true
            }
            Some(_) => false,
            None => {
                self.active = Some(MarkdownFence { marker, length });
                true
            }
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.active.is_some()
    }
}

fn fence_run(line: &str) -> Option<(char, usize, &str)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = line
        .chars()
        .take_while(|candidate| *candidate == marker)
        .count();
    (length >= 3).then(|| (marker, length, &line[length..]))
}

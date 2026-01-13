fn visited_dspan(&mut self, dspan: DelimSpan) -> Span {
        let mut span = dspan.entire();
        self.marker.mark_span(&mut span);
        span
    }
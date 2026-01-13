pub fn set_cursor_visible(&mut self, cursor_visible: bool) {
        self.cursor_visible = cursor_visible;

        if self.cursor_visible {
            match &self.selected_cursor {
                SelectedCursor::Named(icon) => self.set_cursor(*icon),
                SelectedCursor::Custom(cursor) => self.apply_custom_cursor(cursor),
            }
        } else {
            for pointer in self.pointers.iter().filter_map(|pointer| pointer.upgrade()) {
                let latest_enter_serial = pointer.pointer().winit_data().latest_enter_serial();

                pointer.pointer().set_cursor(latest_enter_serial, None, 0, 0);
            }
        }
    }
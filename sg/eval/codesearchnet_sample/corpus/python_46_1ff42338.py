def switch_on(self):
        """Turn the switch on."""
        success = self.set_status(CONST.STATUS_ON_INT)

        if success:
            self._json_state['status'] = CONST.STATUS_ON

        return success
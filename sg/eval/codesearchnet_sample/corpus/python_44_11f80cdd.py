def request_get_variable(self, py_db, seq, thread_id, frame_id, scope, attrs):
        '''
        :param scope: 'FRAME' or 'GLOBAL'
        '''
        int_cmd = InternalGetVariable(seq, thread_id, frame_id, scope, attrs)
        py_db.post_internal_command(int_cmd, thread_id)
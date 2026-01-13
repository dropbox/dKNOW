def object_apply(self, function):
        '''
        Groups by _oid, then applies the function to each group
        and finally concatenates the results.

        :param function: func that takes a DataFrame and returns a DataFrame
        '''
        return pd.concat([function(df) for _, df in self.groupby(self._oid)])
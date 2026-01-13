def save_post(self, title, text, user_id, tags, draft=False,
                  post_date=None, last_modified_date=None, meta_data=None,
                  post_id=None):
        """
        Persist the blog post data. If ``post_id`` is ``None`` or ``post_id``
        is invalid, the post must be inserted into the storage. If ``post_id``
        is a valid id, then the data must be updated.

        :param title: The title of the blog post
        :type title: str
        :param text: The text of the blog post
        :type text: str
        :param user_id: The user identifier
        :type user_id: str
        :param tags: A list of tags
        :type tags: list
        :param draft: (Optional) If the post is a draft of if needs to be
         published. (default ``False``)
        :type draft: bool
        :param post_date: (Optional) The date the blog was posted (default
         datetime.datetime.utcnow() )
        :type post_date: datetime.datetime
        :param last_modified_date: (Optional) The date when blog was last
         modified  (default datetime.datetime.utcnow() )
        :type last_modified_date: datetime.datetime
        :param post_id: (Optional) The post identifier. This should be ``None``
         for an insert call,
         and a valid value for update. (default ``None``)
        :type post_id: str

        :return: The post_id value, in case of a successful insert or update.
         Return ``None`` if there were errors.
        """
        new_post = post_id is None
        post_id = _as_int(post_id)
        current_datetime = datetime.datetime.utcnow()
        draft = 1 if draft is True else 0
        post_date = post_date if post_date is not None else current_datetime
        last_modified_date = last_modified_date if last_modified_date is not \
            None else current_datetime

        with self._engine.begin() as conn:
            try:
                if post_id is not None:  # validate post_id
                    exists_statement = sqla.select([self._post_table]).where(
                        self._post_table.c.id == post_id)
                    exists = \
                        conn.execute(exists_statement).fetchone() is not None
                    post_id = post_id if exists else None
                post_statement = \
                    self._post_table.insert() if post_id is None else \
                    self._post_table.update().where(
                        self._post_table.c.id == post_id)
                post_statement = post_statement.values(
                    title=title, text=text, post_date=post_date,
                    last_modified_date=last_modified_date, draft=draft
                )

                post_result = conn.execute(post_statement)
                post_id = post_result.inserted_primary_key[0] \
                    if post_id is None else post_id
                self._save_tags(tags, post_id, conn)
                self._save_user_post(user_id, post_id, conn)

            except Exception as e:
                self._logger.exception(str(e))
                post_id = None
        return post_id
public function destroy(User $user, Contact $contact)
    {
        if ($user->canDo('contact.contact.delete') && $user->isAdmin()) {
            return true;
        }
    }
function SettingsSelect( params, node ){
		makeLocalSelect(params);
		Select.apply(this, arguments);
		this.mylabel = lib.$(".setting-label", this.node); //$NON-NLS-0$
	}
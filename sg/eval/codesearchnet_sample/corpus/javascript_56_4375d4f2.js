function Observer(fn, observerInfo) {
	// call Subscribable constructor
	Subscribable.call(this);
	
	this.fn = fn;
	this.dependency = [];
	this.currentValue = null;
	this.observerInfo = observerInfo;
	
	// hard binded callbacks
	this._dependencyGetter = this.dependencyGetter.bind(this);
	
	this.call();
}
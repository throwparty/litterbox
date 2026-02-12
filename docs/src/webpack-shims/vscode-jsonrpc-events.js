const Event = {
  None: () => ({
    dispose() {},
  }),
};

class CallbackList {
  add(callback, context) {
    if (!this._callbacks) {
      this._callbacks = [];
      this._contexts = [];
    }
    this._callbacks.push(callback);
    this._contexts.push(context);
  }

  remove(callback, context) {
    if (!this._callbacks) {
      return;
    }
    for (let i = 0; i < this._callbacks.length; i += 1) {
      if (this._callbacks[i] === callback && this._contexts[i] === context) {
        this._callbacks.splice(i, 1);
        this._contexts.splice(i, 1);
        return;
      }
    }
  }

  invoke(event) {
    if (!this._callbacks) {
      return;
    }
    for (let i = 0; i < this._callbacks.length; i += 1) {
      this._callbacks[i].call(this._contexts[i], event);
    }
  }

  isEmpty() {
    return !this._callbacks || this._callbacks.length === 0;
  }

  dispose() {
    this._callbacks = undefined;
    this._contexts = undefined;
  }
}

class Emitter {
  constructor(options) {
    this._options = options;
  }

  get event() {
    if (!this._event) {
      this._event = (listener, thisArgs) => {
        if (!this._callbacks) {
          this._callbacks = new CallbackList();
        }
        if (this._options?.onFirstListenerAdd && this._callbacks.isEmpty()) {
          this._options.onFirstListenerAdd(this);
        }
        this._callbacks.add(listener, thisArgs);
        const result = {
          dispose: () => {
            if (!this._callbacks) {
              return;
            }
            this._callbacks.remove(listener, thisArgs);
            result.dispose = Emitter._noop;
            if (
              this._options?.onLastListenerRemove &&
              this._callbacks.isEmpty()
            ) {
              this._options.onLastListenerRemove(this);
            }
          },
        };
        return result;
      };
    }
    return this._event;
  }

  fire(event) {
    if (this._callbacks) {
      this._callbacks.invoke(event);
    }
  }

  dispose() {
    if (this._callbacks) {
      this._callbacks.dispose();
      this._callbacks = undefined;
    }
  }
}

Emitter._noop = () => {};

export { Emitter, Event };

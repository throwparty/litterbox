import { Emitter, Event } from "./vscode-jsonrpc-events.js";

const CancellationToken = {
  None: Object.freeze({
    isCancellationRequested: false,
    onCancellationRequested: Event.None,
  }),
  Cancelled: Object.freeze({
    isCancellationRequested: true,
    onCancellationRequested: Event.None,
  }),
  is(value) {
    return (
      !!value &&
      (value === CancellationToken.None ||
        value === CancellationToken.Cancelled ||
        (typeof value.isCancellationRequested === "boolean" &&
          typeof value.onCancellationRequested === "function"))
    );
  },
};

const shortcutEvent = Object.freeze((callback, context) => {
  const handle = setTimeout(callback.bind(context), 0);
  return {
    dispose() {
      clearTimeout(handle);
    },
  };
});

class MutableToken {
  constructor() {
    this._isCancelled = false;
  }

  cancel() {
    if (!this._isCancelled) {
      this._isCancelled = true;
      if (this._emitter) {
        this._emitter.fire(undefined);
        this.dispose();
      }
    }
  }

  get isCancellationRequested() {
    return this._isCancelled;
  }

  get onCancellationRequested() {
    if (this._isCancelled) {
      return shortcutEvent;
    }
    if (!this._emitter) {
      this._emitter = new Emitter();
    }
    return this._emitter.event;
  }

  dispose() {
    if (this._emitter) {
      this._emitter.dispose();
      this._emitter = undefined;
    }
  }
}

class CancellationTokenSource {
  get token() {
    if (!this._token) {
      this._token = new MutableToken();
    }
    return this._token;
  }

  cancel() {
    if (!this._token) {
      this._token = CancellationToken.Cancelled;
    } else {
      this._token.cancel();
    }
  }

  dispose() {
    if (!this._token) {
      this._token = CancellationToken.None;
    } else if (this._token instanceof MutableToken) {
      this._token.dispose();
    }
  }
}

export { CancellationToken, CancellationTokenSource };

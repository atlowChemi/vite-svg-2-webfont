// The docs build and serve on Node.js 26, but their tests run on every Node.js the repo supports. Node.js 22 has no
// `AsyncDisposableStack`, so the es-shims one fills it in there; where it's native, the shim leaves it alone.
// TODO [engine:node@>=24, 2027-04-30]: Remove this setup file once Node 22 support ends.
import 'disposablestack/auto';

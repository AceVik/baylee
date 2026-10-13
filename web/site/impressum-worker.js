// The Impressum's proof of work (web/site/impressum.js starts this worker,
// the feedback service checks the answer: crates/baylee-feedback/src/
// impressum.rs). A file of its own because the page's CSP allows no blob:
// or inline worker.
//
// The puzzle, the same in all three places:
//
//   for part = 0 .. parts-1, find the smallest counter c (0, 1, 2, …) with
//   SHA-256( UTF-8( nonce + ":" + part + ":" + c ) )
//   starting with at least `difficulty` zero bits;
//   `nonce` is the challenge's nonce as sent (base64url text), `part` and
//   `c` in decimal. The answer is the list of counters.
//
// Several small puzzles instead of one large one: the same work on average,
// far less spread between a lucky and an unlucky visitor.
//
// SHA-256 is written out here because crypto.subtle answers each hash
// asynchronously, far too slowly for a million of them. Node can load this
// file too (the tests do): the message handler is attached only in a worker.
(function (scope) {
  "use strict";

  var K = new Int32Array([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
  ]);
  var W = new Int32Array(64);
  var H = new Int32Array(8);

  var BUF = new Uint8Array(256);
  var DV = new DataView(BUF.buffer);

  // SHA-256 of the first `len` bytes of BUF (len ≤ 247) into H
  function digest(len) {
    BUF[len] = 0x80;
    var blocks = (len + 9 + 63) >> 6, end = blocks << 6;
    for (var z = len + 1; z < end - 4; z++) BUF[z] = 0;
    DV.setUint32(end - 4, len * 8, false);
    H[0] = 0x6a09e667; H[1] = 0xbb67ae85; H[2] = 0x3c6ef372; H[3] = 0xa54ff53a;
    H[4] = 0x510e527f; H[5] = 0x9b05688c; H[6] = 0x1f83d9ab; H[7] = 0x5be0cd19;
    for (var b = 0; b < blocks; b++) {
      var t;
      for (t = 0; t < 16; t++) W[t] = DV.getInt32((b << 6) + (t << 2), false);
      for (t = 16; t < 64; t++) {
        var x = W[t - 15], y = W[t - 2];
        var s0 = ((x >>> 7) | (x << 25)) ^ ((x >>> 18) | (x << 14)) ^ (x >>> 3);
        var s1 = ((y >>> 17) | (y << 15)) ^ ((y >>> 19) | (y << 13)) ^ (y >>> 10);
        W[t] = (W[t - 16] + s0 + W[t - 7] + s1) | 0;
      }
      var a = H[0], bb = H[1], c = H[2], d = H[3], e = H[4], f = H[5], g = H[6], h = H[7];
      for (t = 0; t < 64; t++) {
        var S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
        var t1 = (h + S1 + ((e & f) ^ (~e & g)) + K[t] + W[t]) | 0;
        var S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
        var t2 = (S0 + ((a & bb) ^ (a & c) ^ (bb & c))) | 0;
        h = g; g = f; f = e; e = (d + t1) | 0; d = c; c = bb; bb = a; a = (t1 + t2) | 0;
      }
      H[0] = (H[0] + a) | 0; H[1] = (H[1] + bb) | 0; H[2] = (H[2] + c) | 0; H[3] = (H[3] + d) | 0;
      H[4] = (H[4] + e) | 0; H[5] = (H[5] + f) | 0; H[6] = (H[6] + g) | 0; H[7] = (H[7] + h) | 0;
    }
    return H;
  }

  // writes `text` (ASCII) into BUF at 0; its length
  function put(text) {
    if (text.length > 200) throw new Error("too long");
    for (var i = 0; i < text.length; i++) BUF[i] = text.charCodeAt(i) & 0x7f;
    return text.length;
  }
  // writes `n` in decimal into BUF at `at`; the new length
  function putNumber(at, n) {
    var digits = 1, m = n;
    while (m >= 10) { m = Math.floor(m / 10); digits++; }
    for (var i = digits - 1; i >= 0; i--) { BUF[at + i] = 48 + (n % 10); n = Math.floor(n / 10); }
    return at + digits;
  }

  // whether the first `bits` bits of H are zero
  function leadingZeros(h, bits) {
    for (var i = 0; bits > 0; i++, bits -= 32) {
      var word = h[i] >>> 0;
      if (bits >= 32) { if (word !== 0) return false; }
      else if ((word >>> (32 - bits)) !== 0) return false;
    }
    return true;
  }

  // the counters for every part; `progress(done, parts)` after each part
  function solve(nonce, parts, difficulty, progress) {
    var counters = [];
    for (var p = 0; p < parts; p++) {
      var start = put(nonce + ":" + p + ":");
      for (var c = 0; ; c++) {
        if (leadingZeros(digest(putNumber(start, c)), difficulty)) { counters.push(c); break; }
      }
      if (progress) progress(p + 1, parts);
    }
    return counters;
  }

  function hex(text) {
    var h = digest(put(text)), s = "";
    for (var i = 0; i < 8; i++) s += ("00000000" + (h[i] >>> 0).toString(16)).slice(-8);
    return s;
  }

  var api = { solve: solve, hex: hex };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  if (typeof scope.postMessage === "function" && typeof scope.importScripts === "function") {
    scope.onmessage = function (e) {
      var m = e.data || {};
      if (m.bench) {
        var start = Date.now(), n = 0;
        var at = put("AAAAAAAAAAAAAAAAAAAAAA:0:");
        while (Date.now() - start < 300) { for (var k = 0; k < 1000; k++) digest(putNumber(at, n++)); }
        scope.postMessage({ bench: n / ((Date.now() - start) / 1000) });
        return;
      }
      var counters = solve(String(m.nonce), m.parts | 0, m.difficulty | 0, function (done, all) {
        scope.postMessage({ progress: done / all });
      });
      scope.postMessage({ counters: counters });
    };
  }
})(typeof self !== "undefined" ? self : this);

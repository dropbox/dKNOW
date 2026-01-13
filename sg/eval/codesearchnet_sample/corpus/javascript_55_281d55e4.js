function getRand(cb) {
  crypto.randomBytes(2, function(err, b) {
    if (err) cb(err)
    else cb(null, b.readUInt16LE(0))
  })
}
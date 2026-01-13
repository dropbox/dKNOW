function _setIndex (arrayLike, idx, value, updater) {
    var result = slice(arrayLike, 0, arrayLike.length);
    var n = _toNaturalIndex(idx, result.length);

    if (n === n) { // eslint-disable-line no-self-compare
        result[n] = arguments.length === 4 ? updater(arrayLike[n]) : value;
    }

    return result;
}
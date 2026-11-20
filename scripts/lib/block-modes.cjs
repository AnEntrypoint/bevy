function totalOf(errors) {
    return errors[0] + errors[1] + errors[2];
}

function chooseBest(options) {
    return options.reduce((best, option) => (totalOf(option.errors) < totalOf(best.errors) ? option : best));
}

module.exports = { totalOf, chooseBest };

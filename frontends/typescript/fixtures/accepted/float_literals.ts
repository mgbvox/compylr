export function float_identity(value: float): float {
    return value;
}

export function float_call(): float {
    return float_identity(3.7);
}

export function float_array(): Array<float> {
    return [1.5, 2.5];
}

export function float_push(): Array<float> {
    let values: Array<float> = [];
    values.push(1.5);
    return values;
}

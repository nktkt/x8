export const square = (n: number): number => n * n;
export const sum = (xs: number[]): number => xs.reduce((a, b) => a + b, 0);
export const mean = (xs: number[]): number => sum(xs) / xs.length;

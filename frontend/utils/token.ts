export interface Token {
    address: string,
    expires: string,
    subject: User
}

export function urlsafeBase64Decode(s: string): string {
    s = s
        .replaceAll("-", "+")
        .replaceAll("_", "/")
    while (s.length % 4) s += '=';
    return atob(s);
}

export function getToken(): Token | undefined {
    const value = document.cookie
        .split("; ")
        .map(part => part.split('='))
        .find(([key, _]) => key == "rustoj-token")
        ?.[1];
    return value ? JSON.parse(urlsafeBase64Decode(value.split('.')[0])) : undefined;
}
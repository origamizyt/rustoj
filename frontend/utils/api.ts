const API_BASE = import.meta.env.DEV ? "http://localhost:12345" : "";
const UPPERCASE_LETTERS = "abcdefghijklmnopqrstuvwxyz".toUpperCase();

function url(path: string): string {
    return `${API_BASE}${path}`;
}

function camelToSnake(name: string): string {
    let snake = "";
    for (const char of name) {
        if (UPPERCASE_LETTERS.includes(char)) {
            snake += "_";
            snake += char.toLowerCase();
        }
        else {
            snake += char;
        }
    }
    return snake;
}

function snakeToCamel(name: string): string {
    let camel = "";
    let nextToUpper = false;
    for (const char of name) {
        if (char == '_') {
            nextToUpper = true;
        }
        else if (nextToUpper) {
            camel += char.toUpperCase();
        }
        else {
            camel += char;
        }
    }
    return camel;
}

function snakeToCamelDeep(obj: any): any {
    if (typeof obj != 'object' || obj instanceof Array) return obj;
    const result: any = {};
    for (const key in obj) {
        result[snakeToCamel(key)] = snakeToCamelDeep(obj[key]);
    }
    return result;
}

function toQueryString<Q extends Record<string, string> = any>(query: Q): string {
    let search = "?";
    for (const key in query) {
        search += camelToSnake(key);
        search += "=";
        search += query[key];
        search += "&";
    }
    return search.slice(0, search.length-1);
}

async function get<T = unknown, Q extends Record<string, string> = any>(path: string, query: Q, json?: boolean): Promise<T> {
    const resp = await fetch(url(path) + toQueryString(query), {
        credentials: 'include'
    });
    if (resp.status == 200) return json ? snakeToCamelDeep(await resp.json()) : undefined;
    else {
        throw await resp.json();
    }
}

async function post<T = unknown>(path: string, body: any, json?: boolean): Promise<T> {
    const resp = await fetch(url(path), {
        method: 'POST',
        credentials: 'include',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(body)
    });
    if (resp.status == 200) return json ? snakeToCamelDeep(await resp.json()) : undefined;
    else {
        throw await resp.json();
    }
}

export interface Problem {
    id: number,
    name: string,
    type: 'standard' | 'strict' | 'spj' | 'dynamic_ranking'
    desc: string,
    cases: number,
    score: number
}

export interface User {
    id: number,
    name: string
}

export interface Ranking {
    user: User,
    rank: number,
    scores: number[]
}

export interface Job {
    id: number,
    createdTime: string,
    updatedTime: string,
    submission: JobRequest,
    state: JobStatus,
    result: Status,
    score: number,
    cases: JobCase[],
}

export interface JobRequest {
    userId: number,
    problemId: number,
    contestId: number,
    sourceCode: string,
    language: string
}

export interface JobCase {
    id: number,
    result: Status,
    time: number,
    memory: number,
    info: string,
}

export type Status = 
    'Waiting' | 'Running' | 'Accepted' | 'Compilation Error' | 'Compilation Success' | 
    'Wrong Answer' | 'Runtime Error' | 'Time Limit Exceeded' | 'Memory Limit Exceeded' | 
    'System Error' | 'SPJ Error' | 'Skipped';

export type JobStatus = 'Queueing' | 'Running' | 'Finished' | 'Canceled';

export interface Failure {
    code: number,
    reason: string,
    message: string
}

const backend = {
    get,
    post
}

export default backend;
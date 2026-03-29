/**
 * Error types for better error handling
 */
export enum ErrorType {
  NETWORK = 'network',
  AUTH = 'auth',
  NOT_FOUND = 'not_found',
  VALIDATION = 'validation',
  UNKNOWN = 'unknown',
}

export interface AppError {
  type: ErrorType;
  message: string;
  code?: string;
  originalError?: Error;
}

export function classifyError(error: unknown): AppError {
  if (error instanceof Error) {
    const message = error.message.toLowerCase();

    if (message.includes('network') || message.includes('connection') || message.includes('timeout')) {
      return { type: ErrorType.NETWORK, message: error.message, originalError: error };
    }
    if (message.includes('auth') || message.includes('password') || message.includes('key')) {
      return { type: ErrorType.AUTH, message: error.message, originalError: error };
    }
    if (message.includes('not found') || message.includes('does not exist')) {
      return { type: ErrorType.NOT_FOUND, message: error.message, originalError: error };
    }
    if (message.includes('invalid') || message.includes('required')) {
      return { type: ErrorType.VALIDATION, message: error.message, originalError: error };
    }
  }

  return {
    type: ErrorType.UNKNOWN,
    message: error instanceof Error ? error.message : String(error),
    originalError: error instanceof Error ? error : undefined,
  };
}

import { Component, ReactNode } from 'react';
import ErrorState from '@/components/states/ErrorState';

type ErrorBoundaryProps = {
  children: ReactNode;
  onRetry?: () => void;
};

type ErrorBoundaryState = {
  hasError: boolean;
  message: string;
};

class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, message: '' };
  }

  // React passes whatever was thrown, which is not always an Error.
  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    const message = error instanceof Error ? error.message : error != null ? String(error) : '';
    return { hasError: true, message: message || 'Unexpected UI error' };
  }

  componentDidCatch(error: unknown) {
    console.error('Error boundary caught component crash', error);
  }

  handleRetry = () => {
    this.setState({ hasError: false, message: '' });
    this.props.onRetry?.();
  };

  render() {
    if (this.state.hasError) {
      return <ErrorState message={this.state.message} onRetry={this.handleRetry} />;
    }
    return this.props.children;
  }
}

export default ErrorBoundary;


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

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, message: error.message || 'Unexpected UI error' };
  }

  componentDidCatch(error: Error) {
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


import React, { useState } from 'react';
import { useAIStore } from '../store/aiStore';
import { ThumbsUp, ThumbsDown, Star } from 'lucide-react';

interface FeedbackPanelProps {
  command: string;
  onClose: () => void;
}

export const FeedbackPanel: React.FC<FeedbackPanelProps> = ({ command, onClose }) => {
  const [rating, setRating] = useState<number>(0);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { updateFeedback } = useAIStore();

  const handleRatingClick = (value: number) => {
    setRating(value);
  };

  const handleSubmit = async () => {
    if (rating === 0) return;
    
    setIsSubmitting(true);
    try {
      await updateFeedback(command, rating / 5); // Convert to 0-1 scale
      onClose();
    } catch (error) {
      console.error('Failed to submit feedback:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleQuickFeedback = async (value: number) => {
    setIsSubmitting(true);
    try {
      await updateFeedback(command, value);
      onClose();
    } catch (error) {
      console.error('Failed to submit feedback:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="bg-terminal-surface border border-terminal-border rounded-lg p-4 shadow-lg">
      <div className="mb-3">
        <h3 className="text-sm font-medium text-terminal-text mb-1">
          How helpful was the AI suggestion?
        </h3>
        <p className="text-xs text-terminal-muted">
          Command: <code className="bg-terminal-bg px-1 rounded">{command}</code>
        </p>
      </div>

      {/* Quick Feedback */}
      <div className="flex items-center space-x-2 mb-3">
        <span className="text-xs text-terminal-muted">Quick:</span>
        <button
          onClick={() => handleQuickFeedback(0.2)}
          disabled={isSubmitting}
          className="flex items-center space-x-1 px-2 py-1 bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded text-xs transition-colors disabled:opacity-50"
        >
          <ThumbsDown className="w-3 h-3" />
          <span>Poor</span>
        </button>
        <button
          onClick={() => handleQuickFeedback(1.0)}
          disabled={isSubmitting}
          className="flex items-center space-x-1 px-2 py-1 bg-green-500/20 hover:bg-green-500/30 text-green-400 rounded text-xs transition-colors disabled:opacity-50"
        >
          <ThumbsUp className="w-3 h-3" />
          <span>Great</span>
        </button>
      </div>

      {/* Detailed Rating */}
      <div className="border-t border-terminal-border pt-3">
        <div className="flex items-center space-x-1 mb-2">
          <span className="text-xs text-terminal-muted">Detailed:</span>
          {[1, 2, 3, 4, 5].map((star) => (
            <button
              key={star}
              onClick={() => handleRatingClick(star)}
              className={`transition-colors ${
                star <= rating ? 'text-yellow-400' : 'text-terminal-muted hover:text-yellow-400'
              }`}
            >
              <Star className="w-4 h-4 fill-current" />
            </button>
          ))}
        </div>

        <div className="flex items-center justify-between">
          <button
            onClick={onClose}
            className="text-xs text-terminal-muted hover:text-terminal-text transition-colors"
          >
            Skip
          </button>
          <button
            onClick={handleSubmit}
            disabled={rating === 0 || isSubmitting}
            className="px-3 py-1 bg-ai-primary hover:bg-ai-primary/80 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs rounded transition-colors"
          >
            {isSubmitting ? 'Submitting...' : 'Submit'}
          </button>
        </div>
      </div>
    </div>
  );
};

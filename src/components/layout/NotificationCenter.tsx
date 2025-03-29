import React from 'react';
import { useNotificationStore } from '../../store/notificationStore';
import { format, formatDistanceToNow } from 'date-fns';
import { XMarkIcon, CheckIcon } from '@heroicons/react/24/outline';
import clsx from 'clsx';

const NotificationCenter: React.FC = () => {
  const {
    notifications,
    showNotificationCenter,
    markAsRead,
    markAllAsRead,
    removeNotification,
  } = useNotificationStore();

  if (!showNotificationCenter) {
    return null;
  }

  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const isToday = date.toDateString() === now.toDateString();
    
    if (isToday) {
      return format(date, 'HH:mm');
    }
    
    return formatDistanceToNow(date, { addSuffix: true });
  };

  const getIconColor = (type: string) => {
    switch (type) {
      case 'success':
        return 'text-success-500';
      case 'warning':
        return 'text-warning-500';
      case 'error':
        return 'text-danger-500';
      default:
        return 'text-primary-500';
    }
  };

  return (
    <div className="fixed right-4 top-16 w-80 max-h-[calc(100vh-5rem)] bg-secondary-800 border border-secondary-700 rounded-lg shadow-lg z-50 flex flex-col">
      <div className="flex items-center justify-between p-4 border-b border-secondary-700">
        <h2 className="text-lg font-semibold text-white">Notifications</h2>
        <button
          onClick={markAllAsRead}
          className="text-sm text-primary-400 hover:text-primary-300"
        >
          Mark all as read
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {notifications.length === 0 ? (
          <div className="p-4 text-center text-secondary-400">
            No notifications
          </div>
        ) : (
          <ul className="divide-y divide-secondary-700">
            {notifications.map((notification) => (
              <li
                key={notification.id}
                className={clsx(
                  'p-4 hover:bg-secondary-700 transition-colors duration-200',
                  !notification.read && 'bg-secondary-750'
                )}
              >
                <div className="flex items-start">
                  <div className={clsx('w-2 h-2 mt-1.5 rounded-full mr-2', getIconColor(notification.type))} />
                  <div className="flex-1 min-w-0">
                    <div className="flex justify-between">
                      <p className="text-sm font-medium text-white truncate">
                        {notification.title}
                      </p>
                      <p className="text-xs text-secondary-400 whitespace-nowrap ml-2">
                        {formatTime(notification.timestamp)}
                      </p>
                    </div>
                    <p className="mt-1 text-sm text-secondary-300">
                      {notification.message}
                    </p>
                    {notification.link && (
                      <a
                        href={notification.link}
                        className="mt-1 text-xs text-primary-400 hover:text-primary-300"
                      >
                        View details
                      </a>
                    )}
                  </div>
                  <div className="flex ml-2">
                    {!notification.read && (
                      <button
                        onClick={() => markAsRead(notification.id)}
                        className="p-1 text-secondary-400 hover:text-primary-400"
                        aria-label="Mark as read"
                      >
                        <CheckIcon className="w-4 h-4" />
                      </button>
                    )}
                    <button
                      onClick={() => removeNotification(notification.id)}
                      className="p-1 text-secondary-400 hover:text-danger-400"
                      aria-label="Remove notification"
                    >
                      <XMarkIcon className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
};

export default NotificationCenter;
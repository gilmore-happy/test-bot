import { create } from 'zustand';
import { Notification } from '../types';

interface NotificationState {
  notifications: Notification[];
  unreadCount: number;
  showNotificationCenter: boolean;
  addNotification: (notification: Omit<Notification, 'id' | 'timestamp' | 'read'>) => void;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  removeNotification: (id: string) => void;
  toggleNotificationCenter: () => void;
}

let nextId = 1;

export const useNotificationStore = create<NotificationState>((set, get) => ({
  notifications: [],
  unreadCount: 0,
  showNotificationCenter: false,
  
  addNotification: (notification) => {
    const newNotification: Notification = {
      id: `notification-${nextId++}`,
      timestamp: new Date().toISOString(),
      read: false,
      ...notification,
    };
    
    set(state => ({
      notifications: [newNotification, ...state.notifications].slice(0, 100), // Keep only the last 100 notifications
      unreadCount: state.unreadCount + 1,
    }));
  },
  
  markAsRead: (id) => {
    set(state => {
      const notifications = state.notifications.map(notification => 
        notification.id === id && !notification.read
          ? { ...notification, read: true }
          : notification
      );
      
      // Count how many notifications were actually marked as read
      const markedCount = state.notifications.filter(n => n.id === id && !n.read).length;
      
      return {
        notifications,
        unreadCount: Math.max(0, state.unreadCount - markedCount),
      };
    });
  },
  
  markAllAsRead: () => {
    set(state => ({
      notifications: state.notifications.map(notification => ({ ...notification, read: true })),
      unreadCount: 0,
    }));
  },
  
  removeNotification: (id) => {
    set(state => {
      const notification = state.notifications.find(n => n.id === id);
      const unreadAdjustment = notification && !notification.read ? 1 : 0;
      
      return {
        notifications: state.notifications.filter(n => n.id !== id),
        unreadCount: Math.max(0, state.unreadCount - unreadAdjustment),
      };
    });
  },
  
  toggleNotificationCenter: () => {
    set(state => ({
      showNotificationCenter: !state.showNotificationCenter,
    }));
  },
}));
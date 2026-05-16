import { createQueue } from './glide-mq-factory.mts'

export const autotagger = createQueue('autotagger')
export const emailNotifications = createQueue('email-notifications')
export const imageProcessing = createQueue('image-processing')

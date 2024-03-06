// This file was generated by [rspc](https://github.com/oscartbeaumont/rspc). Do not edit this file manually.

export type Procedures = {
    queries: 
        { key: "account.fingerprint", input: never, result: string } | 
        { key: "channel.list", input: never, result: Channel[] } | 
        { key: "channel.messages", input: number, result: Message[] } | 
        { key: "settings.theme", input: never, result: Theme } | 
        { key: "user.list", input: never, result: User[] },
    mutations: 
        { key: "account.create", input: CreateAccountArgs, result: null } | 
        { key: "account.login", input: LoginArgs, result: null } | 
        { key: "channel.create", input: CreateChannelArgs, result: Channel } | 
        { key: "channel.send", input: SendMessageArgs, result: null } | 
        { key: "settings.theme", input: Theme, result: null },
    subscriptions: 
        { key: "event", input: never, result: Event }
};

export type CreateAccountArgs = { username: string; passphrase: string }

export type LoginArgs = { username: string }

export type Event = "Tick" | "NotConnected" | { MessageReceived: { author: string; channel_id: string; content: string } }

export type CreateChannelArgs = { title: string }

export type Message = { id: number; content: string; timestamp: string; authorId: number[]; channelId: number }

/**
 * Send a message to the network.
 */
export type SendMessageArgs = { channel_id: number; content: string }

/**
 * The theme of the application.
 */
export type Theme = "Light" | "Dark"

export type User = { id: number[]; username: string }

export type Channel = { id: number; title: string }

// This file was generated by [rspc](https://github.com/oscartbeaumont/rspc). Do not edit this file manually.

export type Procedures = {
    queries: 
        { key: "account.fingerprint", input: never, result: string } | 
        { key: "account.login", input: LoginArgs, result: null } | 
        { key: "channel.list", input: never, result: Channel[] } | 
        { key: "channel.messages", input: number, result: Message[] } | 
        { key: "settings.theme", input: never, result: Theme } | 
        { key: "user.list", input: number, result: User[] },
    mutations: 
        { key: "account.create", input: CreateAccountArgs, result: null } | 
        { key: "channel.create", input: CreateChannelArgs, result: Channel } | 
        { key: "channel.send", input: SendMessageArgs, result: null } | 
        { key: "settings.theme", input: Theme, result: null },
    subscriptions: 
        { key: "event", input: never, result: Event }
};

/**
 * Send a message to the network.
 */
export type SendMessageArgs = { channel_id: number; content: string }

export type Channel = { id: number; title: string; networkId: number }

export type CreateChannelArgs = { title: string; network_id: number }

export type LoginArgs = { username: string }

export type CreateAccountArgs = { username: string; passphrase: string }

/**
 * The theme of the application.
 */
export type Theme = "Light" | "Dark"

export type User = { id: number[]; username: string; networkId: number }

export type Event = "Tick"

export type Message = { id: number; content: string; timestamp: string; authorId: number[]; channelId: number }

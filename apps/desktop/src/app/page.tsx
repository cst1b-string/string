
'use client'
import { Chat } from './components/newChat';
import { createContext, useState } from 'react';

export const themeContext = createContext(
	{lightMode: false,
	setLightMode: (value: boolean) => {}}
);
//export const themeFuncContext = createContext(() => null);

// export function setLightMode(value: boolean) {

// 	lightMode = value;

// }

export default function Home() {

	const [lightMode, setLightMode] = useState(false);

	return (
		<themeContext.Provider value={{lightMode, setLightMode}}>
				<div>

					<Chat />
				</div>
		</themeContext.Provider>
		
	);
}

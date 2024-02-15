'use client'

import ChatSidebar from "@/components/chatSidebar";
import { createContext, useState } from 'react';

export const themeContext = createContext(
	{lightMode: false,
	setLightMode: (value: boolean) => {}}
);

export default function Home () {

	const [lightMode, setLightMode] = useState(false);

  return (
	<themeContext.Provider value={{lightMode, setLightMode}}>
		<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
			<div className="">
				<ChatSidebar />
			</div>

			<div className="col-span-2 text-white font-bold "> 
				Chat window will go here
			</div>
		</div>
	</themeContext.Provider>
  );
}

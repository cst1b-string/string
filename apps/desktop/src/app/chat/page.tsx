'use client'

import React from 'react';

import { faker } from '@faker-js/faker'
import { useVirtualizer, useWindowVirtualizer } from '@tanstack/react-virtual'
import { randomBytes, randomInt } from 'crypto';

const sentences = new Array(1000).fill(true).map(
	() => faker.lorem.sentence()
)

export default function Home() {
	

	const parentRef = React.useRef<HTMLDivElement>(null)
	
	const count = sentences.length
	const virtualizer = useVirtualizer({
		count,
		getScrollElement: () => parentRef.current,
		estimateSize: () => 45,
	})
	
	const items = virtualizer.getVirtualItems()
	

	return (
		<div
        ref={parentRef}
        className="List"
        style={{
        	height: 400,
			width:1000,
          	overflowY: 'auto',
          	contain: 'strict',
        }}
      >
        <div
          style={{
            height: virtualizer.getTotalSize(),
            width: '100%',
            position: 'relative',
          }}
        >
          <div
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              transform: `translateY(${items[0]?.start ?? 0}px)`,
            }}
          >
            {items.map((virtualRow) => (
              <div
                key={virtualRow.key}
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
                className={
                  virtualRow.index % 2 ? 'ListItemOdd' : 'ListItemEven'
                }
              >
                <div style={{ padding: '10px 0' }}>
                  <div>User {virtualRow.index % 2 ? '1' : '2'}</div>
                  <div>{sentences[virtualRow.index]}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
	); //Amaan
}

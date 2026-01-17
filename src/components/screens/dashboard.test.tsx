import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Dashboard } from './dashboard'

describe('Dashboard', () => {
  it('renders dashboard heading', () => {
    render(<Dashboard />)
    expect(screen.getByRole('heading', { name: /Dashboard画面です/i })).toBeInTheDocument()
  })

  it('renders with correct heading level', () => {
    render(<Dashboard />)
    const heading = screen.getByRole('heading', { name: /Dashboard画面です/i })
    expect(heading.tagName).toBe('H1')
  })

  it('applies container styling', () => {
    const { container } = render(<Dashboard />)
    const div = container.querySelector('.container')
    expect(div).toBeInTheDocument()
    expect(div).toHaveClass('mx-auto')
    expect(div).toHaveClass('py-10')
  })

  it('applies heading styling', () => {
    render(<Dashboard />)
    const heading = screen.getByRole('heading', { name: /Dashboard画面です/i })
    expect(heading).toHaveClass('text-3xl')
    expect(heading).toHaveClass('font-bold')
  })

  it('renders without errors', () => {
    expect(() => render(<Dashboard />)).not.toThrow()
  })
})

import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { Progress } from './progress'

describe('Progress', () => {
  it('renders progress bar', () => {
    const { container } = render(<Progress value={50} />)
    const progress = container.querySelector('[role="progressbar"]')
    expect(progress).toBeInTheDocument()
  })

  it('renders with 0% value', () => {
    const { container } = render(<Progress value={0} />)
    const indicator = container.querySelector('.bg-primary')
    expect(indicator).toBeInTheDocument()
    expect(indicator).toHaveStyle({ transform: 'translateX(-100%)' })
  })

  it('renders with 50% value', () => {
    const { container } = render(<Progress value={50} />)
    const indicator = container.querySelector('.bg-primary')
    expect(indicator).toBeInTheDocument()
    expect(indicator).toHaveStyle({ transform: 'translateX(-50%)' })
  })

  it('renders with 100% value', () => {
    const { container } = render(<Progress value={100} />)
    const indicator = container.querySelector('.bg-primary')
    expect(indicator).toBeInTheDocument()
    expect(indicator).toHaveStyle({ transform: 'translateX(-0%)' })
  })

  it('handles undefined value', () => {
    const { container } = render(<Progress />)
    const indicator = container.querySelector('.bg-primary')
    expect(indicator).toBeInTheDocument()
    // undefined の場合は 0 として扱われる
    expect(indicator).toHaveStyle({ transform: 'translateX(-100%)' })
  })

  it('applies default styling', () => {
    const { container } = render(<Progress value={50} />)
    const root = container.querySelector('[role="progressbar"]')
    expect(root).toHaveClass('relative')
    expect(root).toHaveClass('h-2')
    expect(root).toHaveClass('w-full')
    expect(root).toHaveClass('rounded-full')
    expect(root).toHaveClass('bg-secondary')
  })

  it('accepts custom className', () => {
    const { container } = render(<Progress value={50} className="custom-class" />)
    const root = container.querySelector('[role="progressbar"]')
    expect(root).toHaveClass('custom-class')
    expect(root).toHaveClass('relative') // デフォルトのクラスも保持
  })

  it('indicator applies transition', () => {
    const { container } = render(<Progress value={75} />)
    const indicator = container.querySelector('.bg-primary')
    expect(indicator).toHaveClass('transition-all')
  })

  it('handles edge case values', () => {
    // 負の値
    const { container: container1 } = render(<Progress value={-10} />)
    let indicator = container1.querySelector('.bg-primary')
    expect(indicator).toHaveStyle({ transform: 'translateX(-110%)' })

    // 100を超える値（コンポーネントは100以上の値も受け入れる）
    const { container: container2 } = render(<Progress value={150} />)
    indicator = container2.querySelector('.bg-primary')
    // 150の場合、100 - 150 = -50 なので translateX(50%)
    // 実際には表示上は100%と同じになる
    expect(indicator).toBeInTheDocument()
  })
})
